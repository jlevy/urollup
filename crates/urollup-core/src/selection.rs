//! Session discovery, exact current-session detection and hierarchy selection.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::adapters::Ingested;
use super::ledger::entities::{Relationship, Thread};
use super::ledger::identity::{
    AnalyticalId, IdPrefix, IdentityError, IdentityKey, KeyComponent, StoredIdentity,
};

/// A coding agent whose sessions urollup can select.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Agent {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
    /// Pi, recognized for current-session diagnostics but not yet supported.
    Pi,
}

impl Agent {
    /// The stable command-line token.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Pi => "pi",
        }
    }
}

/// Derives the analytical thread ID used by an agent adapter from its native thread key.
///
/// Claude main sessions use the session ID; Claude subagents use `session/agent`; Codex
/// uses the rollout thread ID. Keeping this derivation shared lets lightweight discovery
/// resolve ordinary analytical selectors without decoding complete transcripts.
pub fn agent_thread_identity(
    agent: Agent,
    native_thread_key: &str,
) -> Result<StoredIdentity, IdentityError> {
    StoredIdentity::derive(IdentityKey::new(
        IdPrefix::Thread,
        "agent-thread",
        vec![KeyComponent::text(agent.token()), KeyComponent::text(native_thread_key)],
    ))
}

/// Derives only the analytical ID for an agent-native thread key.
pub fn derive_agent_thread_id(
    agent: Agent,
    native_thread_key: &str,
) -> Result<AnalyticalId, IdentityError> {
    agent_thread_identity(agent, native_thread_key).map(|identity| identity.id)
}

/// Whether a session selection includes only the selected threads or their subagents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    /// Only threads selected directly.
    SelfOnly,
    /// Selected threads and every child reached through a descendant-defining edge.
    Descendants,
}

/// One thread in the discovery index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedSession {
    /// The normalized thread entity.
    pub thread: Thread,
    /// The agent that wrote it.
    pub agent: Agent,
    /// Files whose records establish the thread.
    pub source_paths: Vec<PathBuf>,
}

impl IndexedSession {
    /// The agent-native session ID, read from the thread's native key.
    ///
    /// Claude main sessions use the session ID and Claude subagents `session/agent`, the
    /// native thread key [`agent_thread_identity`] derives from; Codex uses the thread ID.
    /// Threads without a native key, such as inline Claude sidechains, have none.
    pub fn native_id(&self) -> Option<String> {
        let key = &self.thread.native_key;
        match self.agent {
            Agent::Claude => {
                let session = key.get("session_id")?;
                Some(match key.get("agent_id") {
                    Some(agent) => format!("{session}/{agent}"),
                    None => session.clone(),
                })
            }
            Agent::Codex => key.get("thread_id").cloned(),
            Agent::Pi => None,
        }
    }
}

/// Thread identities, source paths and native hierarchy edges across every adapter.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SessionIndex {
    sessions: BTreeMap<AnalyticalId, IndexedSession>,
    relationships: Vec<Relationship>,
}

impl SessionIndex {
    /// Adds one adapter result to the index.
    pub fn add(&mut self, agent: Agent, ingested: &Ingested) -> Result<(), SelectionError> {
        for (id, thread) in &ingested.threads {
            let evidence_sources: BTreeSet<_> =
                thread.evidence.iter().map(|evidence| &evidence.source).collect();
            let mut paths = Vec::new();
            for source in &ingested.sources {
                let establishes_thread = source
                    .snapshot
                    .source
                    .as_ref()
                    .is_some_and(|identity| evidence_sources.contains(&identity.id))
                    // Inline children share the main transcript, so its path selects
                    // the native main session; their analytical IDs select the children.
                    && thread.source.value().map(String::as_str) != Some("inline-sidechain");
                if establishes_thread
                    || source_locator_belongs_to_thread(agent, thread, &source.snapshot.locator)
                {
                    paths.push(source.snapshot.file.path.clone());
                    paths.extend(source.snapshot.twin.iter().map(|twin| twin.path.clone()));
                }
            }
            paths.sort();
            paths.dedup();
            let session = IndexedSession { thread: thread.clone(), agent, source_paths: paths };
            if let Some(existing) = self.sessions.insert(id.clone(), session.clone()) {
                if existing != session {
                    return Err(SelectionError::ConflictingThread(id.clone()));
                }
            }
        }
        self.relationships.extend(ingested.relationships.iter().cloned());
        self.relationships.sort();
        self.relationships.dedup();
        Ok(())
    }

    /// Every indexed session in analytical-ID order.
    pub fn sessions(&self) -> impl Iterator<Item = (&AnalyticalId, &IndexedSession)> {
        self.sessions.iter()
    }

    /// Returns one indexed session.
    pub fn get(&self, id: &AnalyticalId) -> Option<&IndexedSession> {
        self.sessions.get(id)
    }

    /// Resolves a native ID, analytical `thr-` ID or transcript path.
    pub fn resolve(&self, selector: &OsStr) -> Result<AnalyticalId, SelectionError> {
        self.resolve_filtered(selector, None)
    }

    /// Resolves a selector for one agent, as exact current-session detection requires.
    pub fn resolve_for_agent(
        &self,
        selector: &OsStr,
        agent: Agent,
    ) -> Result<AnalyticalId, SelectionError> {
        self.resolve_filtered(selector, Some(agent))
    }

    fn resolve_filtered(
        &self,
        selector: &OsStr,
        agent: Option<Agent>,
    ) -> Result<AnalyticalId, SelectionError> {
        let text = selector.to_str();
        let path = Path::new(selector);
        let canonical = std::fs::canonicalize(path).ok();
        let mut candidates = BTreeSet::new();
        for (id, session) in &self.sessions {
            if agent.is_some_and(|expected| session.agent != expected) {
                continue;
            }
            let analytical = text.is_some_and(|value| value == id.to_string());
            let native = text.is_some_and(|value| native_selectors(session).contains(value));
            let source_path = session.source_paths.iter().any(|source| {
                source == path
                    || canonical.as_ref().is_some_and(|selected| {
                        std::fs::canonicalize(source).is_ok_and(|source| source == *selected)
                    })
            });
            if analytical || native || source_path {
                candidates.insert(id.clone());
            }
        }
        match candidates.len() {
            0 => Err(SelectionError::UnknownSelector(selector.to_string_lossy().into_owned())),
            1 => Ok(candidates
                .into_iter()
                .next()
                .expect("a one-member candidate set has a first member")),
            count => Err(SelectionError::AmbiguousSelector {
                selector: selector.to_string_lossy().into_owned(),
                count,
            }),
        }
    }

    /// Applies session selectors, agent filters and hierarchy scope.
    pub fn select(&self, query: &SelectionQuery) -> Result<BTreeSet<AnalyticalId>, SelectionError> {
        let mut clauses = Vec::new();
        if let Some(current) = &query.current {
            clauses
                .push(BTreeSet::from([self.resolve_for_agent(&current.selector, current.agent)?]));
        }
        if !query.sessions.is_empty() {
            clauses.push(
                query
                    .sessions
                    .iter()
                    .map(|selector| self.resolve(selector))
                    .collect::<Result<_, _>>()?,
            );
        }
        if query.all {
            clauses.push(self.sessions.keys().cloned().collect());
        }
        if !query.agents.is_empty() {
            clauses.push(
                self.sessions
                    .iter()
                    .filter(|(_, session)| query.agents.contains(&session.agent))
                    .map(|(id, _)| id.clone())
                    .collect(),
            );
        }
        let Some(mut selected) = clauses.pop() else {
            return Err(SelectionError::NoSelector);
        };
        for clause in clauses {
            selected = selected.intersection(&clause).cloned().collect();
        }

        let scope = query.scope.unwrap_or_else(|| {
            if query.current.is_some() || !query.sessions.is_empty() {
                Scope::Descendants
            } else {
                Scope::SelfOnly
            }
        });
        if scope == Scope::Descendants {
            selected = self.with_descendants(selected);
        }
        if !query.agents.is_empty() {
            selected.retain(|id| {
                self.sessions.get(id).is_some_and(|session| query.agents.contains(&session.agent))
            });
        }
        Ok(selected)
    }

    fn with_descendants(&self, mut selected: BTreeSet<AnalyticalId>) -> BTreeSet<AnalyticalId> {
        let mut children: BTreeMap<&AnalyticalId, Vec<&AnalyticalId>> = BTreeMap::new();
        for relationship in &self.relationships {
            if relationship.kind.defines_descendants()
                && self.sessions.contains_key(&relationship.from)
                && self.sessions.contains_key(&relationship.to)
            {
                children.entry(&relationship.from).or_default().push(&relationship.to);
            }
        }
        let mut queue: VecDeque<_> = selected.iter().cloned().collect();
        while let Some(parent) = queue.pop_front() {
            for child in children.get(&parent).into_iter().flatten() {
                if selected.insert((*child).clone()) {
                    queue.push_back((*child).clone());
                }
            }
        }
        selected
    }

    /// Resolves and validates a hook input against the indexed transcript.
    pub fn resolve_hook(&self, hook: &HookInput) -> Result<CurrentSession, SelectionError> {
        let path = hook
            .transcript_path
            .as_ref()
            .ok_or(SelectionError::UnsavedSession { agent: hook.agent_hint })?;
        let id = self.resolve_for_agent(path.as_os_str(), hook.agent_hint)?;
        let session =
            self.sessions.get(&id).expect("a resolved analytical ID is present in the index");
        let matches = match session.agent {
            Agent::Claude => {
                session.thread.native_key.get("session_id") == Some(&hook.session_id)
                    && hook.agent_id.as_ref().is_none_or(|agent| {
                        session.thread.native_key.get("agent_id") == Some(agent)
                    })
            }
            Agent::Codex => {
                let expected = hook.agent_id.as_ref().unwrap_or(&hook.session_id);
                session.thread.native_key.get("thread_id") == Some(expected)
            }
            Agent::Pi => false,
        };
        if !matches {
            return Err(SelectionError::HookMismatch);
        }
        Ok(CurrentSession { agent: session.agent, selector: OsString::from(id.to_string()) })
    }
}

fn source_locator_belongs_to_thread(agent: Agent, thread: &Thread, locator: &str) -> bool {
    match agent {
        Agent::Codex => thread
            .native_key
            .get("thread_id")
            .is_some_and(|id| locator.strip_prefix(id).is_some_and(|tail| tail.starts_with('/'))),
        Agent::Claude | Agent::Pi => false,
    }
}

fn native_selectors(session: &IndexedSession) -> BTreeSet<&str> {
    let mut selectors = BTreeSet::new();
    match session.agent {
        Agent::Claude => {
            if let Some(agent) = session.thread.native_key.get("agent_id") {
                selectors.insert(agent.as_str());
            } else if let Some(session) = session.thread.native_key.get("session_id") {
                selectors.insert(session.as_str());
            }
        }
        Agent::Codex => {
            if let Some(thread) = session.thread.native_key.get("thread_id") {
                selectors.insert(thread.as_str());
            }
        }
        Agent::Pi => {}
    }
    selectors
}

/// A current-session signal whose selector still needs resolution in the index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentSession {
    /// Which agent supplied the signal.
    pub agent: Agent,
    /// Native session or thread ID, analytical ID, or transcript path.
    pub selector: OsString,
}

/// The process environment relevant to exact current-session detection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CurrentEnvironment {
    variables: BTreeMap<String, OsString>,
}

impl CurrentEnvironment {
    /// Captures current-session variables from the process.
    pub fn from_process() -> Self {
        let variables = [
            "CLAUDE_CODE_SESSION_ID",
            "CODEX_THREAD_ID",
            "CODEX_SESSION_ID",
            "PI_SESSION_ID",
            "PI_SESSION_FILE",
        ]
        .into_iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (name.to_owned(), value)))
        .collect();
        Self { variables }
    }

    /// Builds a hermetic environment from explicit values.
    pub fn new(variables: impl IntoIterator<Item = (impl Into<String>, OsString)>) -> Self {
        Self {
            variables: variables.into_iter().map(|(name, value)| (name.into(), value)).collect(),
        }
    }

    /// Detects the current session without guessing.
    ///
    /// An empty `allowed` set admits every agent; otherwise only named agents count.
    pub fn detect(&self, allowed: &BTreeSet<Agent>) -> Result<CurrentSession, SelectionError> {
        let admits = |agent| allowed.is_empty() || allowed.contains(&agent);
        let value = |name: &str| self.variables.get(name).filter(|value| !value.is_empty());
        let mut candidates = Vec::new();
        if admits(Agent::Claude) {
            if let Some(id) = value("CLAUDE_CODE_SESSION_ID") {
                candidates.push(CurrentSession { agent: Agent::Claude, selector: id.clone() });
            }
        }
        if admits(Agent::Codex) {
            if let Some(id) = value("CODEX_THREAD_ID") {
                candidates.push(CurrentSession { agent: Agent::Codex, selector: id.clone() });
            }
        }
        if admits(Agent::Pi) {
            let file = value("PI_SESSION_FILE");
            let id = value("PI_SESSION_ID");
            if let Some(selector) = file.or(id) {
                candidates.push(CurrentSession { agent: Agent::Pi, selector: selector.clone() });
            }
        }
        if candidates.len() > 1 {
            return Err(SelectionError::AmbiguousCurrent(
                candidates.iter().map(|candidate| candidate.agent).collect(),
            ));
        }
        let Some(current) = candidates.pop() else {
            return Err(SelectionError::CurrentNotDetected);
        };
        if current.agent == Agent::Pi {
            if value("PI_SESSION_FILE").is_none() {
                return Err(SelectionError::UnsavedSession { agent: Agent::Pi });
            }
            return Err(SelectionError::UnsupportedAgent(Agent::Pi));
        }
        Ok(current)
    }
}

/// A normalized subset of Claude Code or Codex hook input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookInput {
    session_id: String,
    agent_id: Option<String>,
    transcript_path: Option<PathBuf>,
    agent_hint: Agent,
}

impl HookInput {
    /// Parses hook input and selects `agent_transcript_path` for `SubagentStop`.
    pub fn parse(bytes: &[u8], agent_hint: Agent) -> Result<Self, SelectionError> {
        if agent_hint == Agent::Pi {
            return Err(SelectionError::UnsupportedAgent(Agent::Pi));
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|source| SelectionError::HookJson { source })?;
        let session_id = value
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            .ok_or(SelectionError::HookField("session_id"))?
            .to_owned();
        let agent_id = value.get("agent_id").and_then(serde_json::Value::as_str).map(str::to_owned);
        let path_field = if value.get("hook_event_name").and_then(serde_json::Value::as_str)
            == Some("SubagentStop")
        {
            "agent_transcript_path"
        } else {
            "transcript_path"
        };
        let transcript_path = match value.get(path_field) {
            Some(serde_json::Value::String(path)) => Some(PathBuf::from(path)),
            Some(serde_json::Value::Null) => None,
            Some(_) | None => return Err(SelectionError::HookField(path_field)),
        };
        Ok(Self { session_id, agent_id, transcript_path, agent_hint })
    }
}

/// Session selectors shared by report commands.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionQuery {
    /// Exact current-session signal, when requested.
    pub current: Option<CurrentSession>,
    /// Native IDs, analytical IDs or transcript paths; repeated selectors form a union.
    pub sessions: Vec<OsString>,
    /// Select every indexed session.
    pub all: bool,
    /// Agent filters; repeated agents form a union and intersect other flags.
    pub agents: BTreeSet<Agent>,
    /// Explicit hierarchy scope, or the design default when omitted.
    pub scope: Option<Scope>,
}

/// Session detection or selector resolution failed.
#[derive(Debug, thiserror::Error)]
pub enum SelectionError {
    /// Two inputs derived the same analytical ID for different threads.
    #[error("conflicting threads derive the same analytical ID {0}")]
    ConflictingThread(AnalyticalId),
    /// No indexed session matches a selector.
    #[error("session selector {0:?} did not match any discovered session")]
    UnknownSelector(String),
    /// More than one indexed session matches a selector.
    #[error("session selector {selector:?} matches {count} discovered sessions")]
    AmbiguousSelector {
        /// The selector.
        selector: String,
        /// Number of matches.
        count: usize,
    },
    /// No selection clause was supplied.
    #[error("no session selector was supplied")]
    NoSelector,
    /// No current-session signal was present.
    #[error("current session was not detected; use --session or --all")]
    CurrentNotDetected,
    /// Several nested-agent signals were present.
    #[error("current session is ambiguous among agents {0:?}; use --agent")]
    AmbiguousCurrent(Vec<Agent>),
    /// The agent is recognized but its adapter has not shipped.
    #[error("the {} session dialect is not supported by this build", .0.token())]
    UnsupportedAgent(Agent),
    /// The detected agent session has no persistent transcript.
    #[error("the detected {} session is unsaved", .agent.token())]
    UnsavedSession {
        /// The agent.
        agent: Agent,
    },
    /// Hook input is not valid JSON.
    #[error("cannot parse hook input: {source}")]
    HookJson {
        /// The JSON decoder error.
        #[source]
        source: serde_json::Error,
    },
    /// A required hook field is missing or has the wrong type.
    #[error("hook input field {0:?} is missing or invalid")]
    HookField(&'static str),
    /// Hook IDs disagree with the transcript they name.
    #[error("hook session fields do not match the selected transcript")]
    HookMismatch,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    use super::{
        Agent, CurrentEnvironment, HookInput, Scope, SelectionError, SelectionQuery, SessionIndex,
    };
    use crate::adapters::{claude_project, codex_rollout};

    fn fixture(dialect: &str, case: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(dialect).join(case)
    }

    fn index() -> SessionIndex {
        let claude = claude_project::ingest_root(&fixture("claude-project", "workflow-subagents"))
            .expect("Claude fixture ingests");
        let codex = codex_rollout::ingest_root(&fixture("codex-rollout", "token-usage-records"))
            .expect("Codex fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Claude, &claude).expect("Claude sessions index");
        index.add(Agent::Codex, &codex).expect("Codex sessions index");
        index
    }

    #[test]
    fn native_analytical_and_path_selectors_resolve_one_session() {
        let index = index();
        let native = OsString::from("00000000-0000-4000-8000-001100000001");
        let id = index.resolve(&native).expect("native ID resolves");
        assert_eq!(index.resolve(OsStr::new(&id.to_string())).expect("thr ID resolves"), id);
        let path = index.get(&id).expect("session exists").source_paths[0].as_os_str();
        assert_eq!(index.resolve(path).expect("path resolves"), id);
    }

    #[test]
    fn session_selections_default_to_descendants_and_all_defaults_to_self() {
        let index = index();
        let parent = OsString::from("00000000-0000-4000-8000-001100000001");
        let descendants = index
            .select(&SelectionQuery { sessions: vec![parent.clone()], ..SelectionQuery::default() })
            .expect("selection succeeds");
        assert_eq!(descendants.len(), 3);

        let own = index
            .select(&SelectionQuery {
                sessions: vec![parent],
                scope: Some(Scope::SelfOnly),
                ..SelectionQuery::default()
            })
            .expect("selection succeeds");
        assert_eq!(own.len(), 1);

        let all = index
            .select(&SelectionQuery { all: true, ..SelectionQuery::default() })
            .expect("selection succeeds");
        assert_eq!(all.len(), index.sessions().count());
    }

    #[test]
    fn different_filter_kinds_intersect_and_repeated_sessions_form_a_union() {
        let index = index();
        let selected = index
            .select(&SelectionQuery {
                sessions: vec![
                    OsString::from("00000000-0000-4000-8000-001100000001"),
                    OsString::from("019f0000-0000-7000-8000-000500000002"),
                ],
                agents: BTreeSet::from([Agent::Claude]),
                scope: Some(Scope::SelfOnly),
                ..SelectionQuery::default()
            })
            .expect("selection succeeds");
        assert_eq!(selected.len(), 1);
        assert_eq!(
            index.get(selected.first().expect("one selection")).expect("indexed").agent,
            Agent::Claude
        );
    }

    #[test]
    fn current_environment_detects_exactly_one_agent() {
        let only_claude =
            CurrentEnvironment::new([("CLAUDE_CODE_SESSION_ID", OsString::from("claude-session"))]);
        let current = only_claude.detect(&BTreeSet::new()).expect("Claude is detected");
        assert_eq!(current.agent, Agent::Claude);

        let nested = CurrentEnvironment::new([
            ("CLAUDE_CODE_SESSION_ID", OsString::from("claude-session")),
            ("CODEX_THREAD_ID", OsString::from("codex-thread")),
        ]);
        assert!(matches!(
            nested.detect(&BTreeSet::new()),
            Err(SelectionError::AmbiguousCurrent(_))
        ));
        let codex =
            nested.detect(&BTreeSet::from([Agent::Codex])).expect("agent filter selects Codex");
        assert_eq!(codex.agent, Agent::Codex);
    }

    #[test]
    fn pi_detection_distinguishes_unsaved_from_unsupported() {
        let unsaved = CurrentEnvironment::new([("PI_SESSION_ID", OsString::from("pi-session"))]);
        assert!(matches!(
            unsaved.detect(&BTreeSet::new()),
            Err(SelectionError::UnsavedSession { agent: Agent::Pi })
        ));
        let persisted = CurrentEnvironment::new([
            ("PI_SESSION_ID", OsString::from("pi-session")),
            ("PI_SESSION_FILE", OsString::from("/tmp/pi-session.jsonl")),
        ]);
        assert!(matches!(
            persisted.detect(&BTreeSet::new()),
            Err(SelectionError::UnsupportedAgent(Agent::Pi))
        ));
    }

    #[test]
    fn subagent_stop_uses_the_agent_transcript_and_validates_ids() {
        let index = index();
        let child = index
            .sessions()
            .map(|(_, session)| session)
            .find(|session| {
                session.agent == Agent::Claude && session.thread.native_key.contains_key("agent_id")
            })
            .expect("fixture has a Claude subagent");
        let child_path = child.source_paths[0].to_string_lossy();
        let agent_id = child.thread.native_key["agent_id"].clone();
        let session_id = child.thread.native_key["session_id"].clone();
        let json = serde_json::json!({
            "hook_event_name": "SubagentStop",
            "session_id": session_id,
            "agent_id": agent_id,
            "transcript_path": "/wrong/parent.jsonl",
            "agent_transcript_path": child_path,
        });
        let hook = HookInput::parse(json.to_string().as_bytes(), Agent::Claude)
            .expect("hook input parses");
        let current = index.resolve_hook(&hook).expect("hook resolves");
        assert_eq!(current.agent, Agent::Claude);
    }

    #[test]
    fn native_ids_name_sessions_subagents_and_codex_threads() {
        let index = index();
        let native_ids: BTreeSet<_> = index
            .sessions()
            .map(|(id, session)| {
                let native = session.native_id().expect("fixture threads have native keys");
                let derived = super::derive_agent_thread_id(session.agent, &native)
                    .expect("native IDs derive analytical IDs");
                assert_eq!(&derived, id, "the native ID is the key the thread ID derives from");
                (session.agent, native)
            })
            .collect();
        assert!(
            native_ids
                .contains(&(Agent::Claude, "00000000-0000-4000-8000-001100000001".to_owned()))
        );
        assert!(native_ids.iter().any(|(agent, native)| *agent == Agent::Claude
            && native.starts_with("00000000-0000-4000-8000-001100000001/")));
        assert!(
            native_ids.contains(&(Agent::Codex, "019f0000-0000-7000-8000-000500000002".to_owned()))
        );

        let inline = claude_project::ingest_root(&fixture("claude-project", "inline-sidechains"))
            .expect("Claude fixture ingests");
        let mut inline_index = SessionIndex::default();
        inline_index.add(Agent::Claude, &inline).expect("Claude sessions index");
        let without: Vec<_> = inline_index
            .sessions()
            .filter(|(_, session)| {
                session.thread.source.value().map(String::as_str) == Some("inline-sidechain")
            })
            .map(|(_, session)| session.native_id())
            .collect();
        assert!(!without.is_empty(), "fixture has inline sidechains");
        assert!(without.iter().all(Option::is_none), "inline sidechains have no native ID");
    }

    #[test]
    fn every_codex_rollout_path_resolves_to_its_thread() {
        let codex = codex_rollout::ingest_root(&fixture("codex-rollout", "revert-file"))
            .expect("Codex fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Codex, &codex).expect("Codex sessions index");
        for source in &codex.sources {
            let selected = index
                .resolve(source.snapshot.file.path.as_os_str())
                .expect("every rollout path resolves");
            assert_eq!(index.get(&selected).expect("thread is indexed").agent, Agent::Codex);
        }
    }
}
