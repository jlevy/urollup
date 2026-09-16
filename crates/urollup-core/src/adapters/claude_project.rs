//! Claude Code project transcript adapter (`claude-project`).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{AdapterError, Ingested};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{
    Basis, Confidence, ModelBasis, ModelName, ModelUsage, ProviderLimitObservation, Relationship,
    RelationshipKind, SourceArtifact, SourceCapability, Thread,
};
use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent, StoredIdentity};
use crate::ledger::reconcile::{
    ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation, RevisionChoice,
    RevisionSelector, reconcile,
};
use crate::ledger::scope::{ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec};
use crate::ledger::tokens::{TokenMeasures, TokenUsage};
use crate::sources::decode::{parse_record, parse_timestamp, text, unsigned};
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::SnapshotManifest;
use crate::sources::reader::{RawRecord, ReadOptions, RecordDisposition, SourceSpec, read_source};
use crate::sources::roots::discover;

const DIALECT: &str = "claude-project";
const AGENT_NAMESPACE: &str = "claude";
const PROVIDER_NAMESPACE: &str = "anthropic";

const REQUEST_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("provider", ComponentRole::Namespace),
    ComponentSlot::required("native_id", ComponentRole::NativeId),
];

const RESPONSE_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "provider-response",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Provider,
    slots: REQUEST_SLOTS,
};

const REQUEST_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "provider-request",
    precedence: 1,
    basis: IdentityBasis::Native,
    scope: IdScope::Provider,
    slots: REQUEST_SLOTS,
};

const AMBIGUOUS_RESPONSE_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "agent-response",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Agent,
    slots: REQUEST_SLOTS,
};

#[derive(Clone)]
struct ParsedRecord {
    evidence: EvidenceRef,
    value: Value,
    source_thread: NativeThread,
    forced_copy: bool,
    request_record: bool,
}

#[derive(Clone)]
struct SourceFacts {
    thread: NativeThread,
    evidence: Option<EvidenceRef>,
    version: Option<String>,
    project: Option<String>,
}

#[derive(Clone)]
struct SubagentMeta {
    child: NativeThread,
    tool_use_id: Option<String>,
    agent_type: Option<String>,
    evidence: EvidenceRef,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NativeThread {
    session: String,
    agent: Option<String>,
}

impl NativeThread {
    fn native_id(&self) -> String {
        match &self.agent {
            Some(agent) => format!("{}/{agent}", self.session),
            None => self.session.clone(),
        }
    }

    fn identity(&self) -> Result<StoredIdentity, AdapterError> {
        Ok(StoredIdentity::derive(IdentityKey::new(
            IdPrefix::Thread,
            "agent-thread",
            vec![KeyComponent::text(AGENT_NAMESPACE), KeyComponent::text(self.native_id())],
        ))?)
    }
}

/// Reads one Claude Code configuration root, including `projects/` and subagents.
pub fn ingest_root(root: &Path) -> Result<Ingested, AdapterError> {
    ingest_roots(&[root.to_owned()], true)
}

/// Reads Claude Code transcript roots selected by discovery.
///
/// Missing variable- or flag-selected roots are errors; missing conventional defaults
/// are skipped.
pub fn ingest_roots(roots: &[PathBuf], missing_is_error: bool) -> Result<Ingested, AdapterError> {
    let discovery = discover(roots);
    if let (true, Some(root)) = (missing_is_error, discovery.missing_roots.first()) {
        return Err(AdapterError::MissingRoot(root.clone()));
    }
    if let Some(unreadable) = discovery.unreadable.first() {
        return Err(AdapterError::UnreadablePath {
            path: unreadable.path.clone(),
            kind: unreadable.kind,
        });
    }

    let mut records = Vec::new();
    let mut source_facts = Vec::new();
    let mut subagent_meta = Vec::new();
    let mut manifest =
        SnapshotManifest { entries: Vec::new(), skipped_links: discovery.skipped_links };
    for source in discovery.sources {
        let source_thread = thread_from_path(&source.locator);
        let mut facts = SourceFacts {
            thread: source_thread.clone(),
            evidence: None,
            version: None,
            project: None,
        };
        let spec = SourceSpec {
            environment: "local",
            dialect: DIALECT,
            locator: &source.locator,
            stable_locator: false,
        };
        let path = source
            .files
            .primary()
            .map_or_else(|| source.root.join(&source.locator), |(path, _)| path.to_owned());
        let entry = read_source(&spec, &source.files, &ReadOptions::default(), |raw| {
            decode_record(raw, &source_thread, &mut facts, &mut records)
        })
        .map_err(|source| AdapterError::Read { path: path.clone(), source })?;
        if source_thread.agent.is_some() {
            if let (Some(identity), Some(meta)) =
                (entry.source.as_ref(), read_subagent_meta(&path)?)
            {
                subagent_meta.push(SubagentMeta {
                    child: source_thread,
                    tool_use_id: text(&meta, &["toolUseId"]).map(str::to_owned),
                    agent_type: text(&meta, &["agentType"]).map(str::to_owned),
                    evidence: EvidenceRef { source: identity.id.clone(), offset: 0, length: 0 },
                });
            }
        }
        manifest.entries.push(entry);
        source_facts.push(facts);
    }

    normalize(&records, &source_facts, &subagent_meta, manifest)
}

fn read_subagent_meta(transcript: &Path) -> Result<Option<Value>, AdapterError> {
    let transcript = if transcript.extension() == Some(std::ffi::OsStr::new("zst")) {
        transcript.with_extension("")
    } else {
        transcript.to_owned()
    };
    let path = transcript.with_extension("meta.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(AdapterError::MetadataRead { path, source }),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| AdapterError::MetadataParse { path, source })
}

fn decode_record(
    raw: &RawRecord<'_>,
    source_thread: &NativeThread,
    facts: &mut SourceFacts,
    records: &mut Vec<ParsedRecord>,
) -> RecordDisposition {
    if facts.evidence.is_none() {
        facts.evidence = Some(raw.evidence.clone());
    }
    let Ok(value) = parse_record(raw.bytes) else {
        return RecordDisposition::Malformed;
    };
    if facts.version.is_none() {
        facts.version = text(&value, &["version"]).map(str::to_owned);
    }
    if facts.project.is_none() {
        facts.project = text(&value, &["cwd"])
            .and_then(|cwd| Path::new(cwd).file_name())
            .map(|name| name.to_string_lossy().into_owned());
    }
    let (value, forced_copy, request_record) = match text(&value, &["type"]) {
        Some("assistant") if unsigned(&value, &["message", "usage", "output_tokens"]).is_some() => {
            let synthetic_error = text(&value, &["message", "model"]) == Some("<synthetic>")
                && text(&value, &["requestId"]).is_none();
            (value, false, !synthetic_error)
        }
        Some("progress") => {
            let Some(mut nested) = value.pointer("/data/message").cloned() else {
                return RecordDisposition::Skipped;
            };
            if unsigned(&nested, &["message", "usage", "output_tokens"]).is_none() {
                return RecordDisposition::Skipped;
            }
            if let (Some(session), Some(object)) =
                (text(&value, &["sessionId"]), nested.as_object_mut())
            {
                object.insert("sessionId".to_owned(), Value::String(session.to_owned()));
            }
            (nested, true, true)
        }
        Some(_) | None => return RecordDisposition::Skipped,
    };
    records.push(ParsedRecord {
        evidence: raw.evidence.clone(),
        value,
        source_thread: source_thread.clone(),
        forced_copy,
        request_record,
    });
    RecordDisposition::Decoded
}

fn normalize(
    records: &[ParsedRecord],
    source_facts: &[SourceFacts],
    subagent_meta: &[SubagentMeta],
    manifest: SnapshotManifest,
) -> Result<Ingested, AdapterError> {
    let mut source_versions: BTreeMap<AnalyticalId, String> = BTreeMap::new();
    for facts in source_facts {
        if let (Some(evidence), Some(version)) = (&facts.evidence, &facts.version) {
            source_versions.entry(evidence.source.clone()).or_insert_with(|| version.clone());
        }
    }
    let mut message_owners: BTreeMap<String, NativeThread> = BTreeMap::new();
    let mut uuid_owners: BTreeMap<String, NativeThread> = BTreeMap::new();
    for record in records.iter().filter(|record| record.request_record && !record.forced_copy) {
        if let Some(message_id) = text(&record.value, &["message", "id"]) {
            message_owners
                .entry(message_id.to_owned())
                .or_insert_with(|| record.source_thread.clone());
        }
        if let Some(uuid) = text(&record.value, &["uuid"]) {
            uuid_owners.entry(uuid.to_owned()).or_insert_with(|| record.source_thread.clone());
        }
    }
    let mut message_models: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for record in records {
        if !record.request_record || is_replayed_record(record, &message_owners, &uuid_owners) {
            continue;
        }
        if let (Some(message_id), Some(model)) =
            (text(&record.value, &["message", "id"]), text(&record.value, &["message", "model"]))
        {
            message_models.entry(message_id.to_owned()).or_default().insert(model.to_owned());
        }
    }
    let ambiguous_messages: BTreeSet<String> = message_models
        .into_iter()
        .filter_map(|(message, models)| (models.len() > 1).then_some(message))
        .collect();
    let mut native_threads: BTreeSet<NativeThread> =
        source_facts.iter().map(|facts| facts.thread.clone()).collect();
    let mut relationships = Vec::new();
    for record in records {
        native_threads.insert(record.source_thread.clone());
        let recorded_session =
            text(&record.value, &["sessionId"]).unwrap_or(&record.source_thread.session).to_owned();
        native_threads.insert(NativeThread { session: recorded_session.clone(), agent: None });
        if record.source_thread.agent.is_none() && recorded_session != record.source_thread.session
        {
            relationships.push((
                RelationshipKind::Fork,
                NativeThread { session: recorded_session, agent: None },
                record.source_thread.clone(),
                record.evidence.clone(),
            ));
        }
    }

    let mut tool_owners = BTreeMap::new();
    for record in records {
        if let Some(content) = record.value.pointer("/message/content").and_then(Value::as_array) {
            for block in content {
                if let (Some("tool_use"), Some(tool_use_id)) =
                    (text(block, &["type"]), text(block, &["id"]))
                {
                    tool_owners
                        .entry(tool_use_id.to_owned())
                        .or_insert_with(|| record.source_thread.clone());
                }
            }
        }
    }
    let purpose_by_thread: BTreeMap<NativeThread, String> = subagent_meta
        .iter()
        .filter_map(|meta| meta.agent_type.clone().map(|purpose| (meta.child.clone(), purpose)))
        .collect();
    let mut spawned = BTreeSet::new();
    for meta in subagent_meta {
        let parent = meta
            .tool_use_id
            .as_ref()
            .and_then(|tool_use_id| tool_owners.get(tool_use_id))
            .cloned()
            .unwrap_or_else(|| NativeThread { session: meta.child.session.clone(), agent: None });
        spawned.insert(meta.child.clone());
        relationships.push((
            RelationshipKind::Spawn,
            parent,
            meta.child.clone(),
            meta.evidence.clone(),
        ));
    }
    for child in native_threads.iter().filter(|thread| thread.agent.is_some()) {
        if let (true, Some(evidence)) = (
            spawned.insert(child.clone()),
            records
                .iter()
                .find(|record| record.source_thread == *child)
                .map(|record| record.evidence.clone()),
        ) {
            relationships.push((
                RelationshipKind::Spawn,
                NativeThread { session: child.session.clone(), agent: None },
                child.clone(),
                evidence,
            ));
        }
    }

    let mut thread_evidence: BTreeMap<NativeThread, Vec<EvidenceRef>> = BTreeMap::new();
    let mut project_by_thread: BTreeMap<NativeThread, String> = BTreeMap::new();
    for facts in source_facts {
        if let Some(evidence) = &facts.evidence {
            thread_evidence.entry(facts.thread.clone()).or_default().push(evidence.clone());
        }
        if let Some(project) = &facts.project {
            project_by_thread.entry(facts.thread.clone()).or_insert_with(|| project.clone());
        }
    }
    for record in records {
        thread_evidence
            .entry(record.source_thread.clone())
            .or_default()
            .push(record.evidence.clone());
        if let Some(project) = text(&record.value, &["cwd"])
            .and_then(|cwd| Path::new(cwd).file_name())
            .map(|name| name.to_string_lossy().into_owned())
        {
            project_by_thread.entry(record.source_thread.clone()).or_insert(project);
        }
    }
    for evidence in thread_evidence.values_mut() {
        evidence.sort();
        evidence.dedup();
    }

    let mut ids = BTreeMap::new();
    let mut threads = BTreeMap::new();
    for native in native_threads {
        let identity = native.identity()?;
        ids.insert(native.clone(), identity.id.clone());
        let mut native_key = BTreeMap::new();
        native_key.insert("session_id".to_owned(), native.session.clone());
        if let Some(agent) = &native.agent {
            native_key.insert("agent_id".to_owned(), agent.clone());
        }
        threads.insert(
            identity.id.clone(),
            Thread {
                identity,
                basis: IdentityBasis::Native,
                aliases: Vec::new(),
                native_key,
                source: Basis::Observed(if native.agent.is_some() {
                    "subagent".to_owned()
                } else {
                    "cli".to_owned()
                }),
                initiator: Basis::Unknown,
                purpose: purpose_by_thread
                    .get(&native)
                    .cloned()
                    .map_or(Basis::Unknown, Basis::Observed),
                execution_environment: Basis::Observed("local".to_owned()),
                project: project_by_thread
                    .get(&native)
                    .cloned()
                    .map_or(Basis::Unknown, Basis::Observed),
                account: Basis::Unknown,
                evidence: thread_evidence.remove(&native).unwrap_or_default(),
            },
        );
    }

    let relationships = relationships
        .into_iter()
        .filter_map(|(kind, from, to, evidence)| {
            Some(Relationship {
                kind,
                from: ids.get(&from)?.clone(),
                to: ids.get(&to)?.clone(),
                confidence: Confidence::Proven,
                evidence: vec![evidence],
            })
        })
        .collect();

    let mut observations = Vec::new();
    let mut limit_observations = Vec::new();
    let mut diagnostics = Vec::new();
    for message_id in &ambiguous_messages {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::ConflictingSharedKey,
            None,
            records
                .iter()
                .filter(|record| {
                    text(&record.value, &["message", "id"]) == Some(message_id.as_str())
                        && !is_replayed_record(record, &message_owners, &uuid_owners)
                })
                .map(|record| record.evidence.clone()),
            format!("Claude message ID {message_id} is reused by conflicting responses"),
        ));
    }
    for record in records {
        append_limits(record, &ids, &ambiguous_messages, &mut limit_observations)?;
        if !record.request_record {
            continue;
        }
        if let Some((flat, breakdown)) = cache_breakdown_mismatch(&record.value) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::UsageInconsistency,
                None,
                [record.evidence.clone()],
                format!(
                    "Claude cache creation total {flat} differs from its lifetime breakdown {breakdown}"
                ),
            ));
        }
        let message_id = text(&record.value, &["message", "id"]);
        let request_id = text(&record.value, &["requestId"]);
        let mut observation = RequestObservation::new(record.evidence.clone(), DIALECT);
        let recorded_session =
            text(&record.value, &["sessionId"]).unwrap_or(&record.source_thread.session).to_owned();
        if let Some(message_id) = message_id {
            let key = if ambiguous_messages.contains(message_id) {
                AMBIGUOUS_RESPONSE_KEY.key(vec![
                    KeyComponent::text(AGENT_NAMESPACE),
                    KeyComponent::text(format!("{recorded_session}/{message_id}")),
                ])?
            } else {
                RESPONSE_KEY.key(vec![
                    KeyComponent::text(PROVIDER_NAMESPACE),
                    KeyComponent::text(message_id),
                ])?
            };
            observation.keys.push(key);
            observation.native_response_id = Some(message_id.to_owned());
        }
        if let Some(request_id) = request_id {
            observation.keys.push(REQUEST_KEY.key(vec![
                KeyComponent::text(PROVIDER_NAMESPACE),
                KeyComponent::text(request_id),
            ])?);
            observation.native_request_id = Some(request_id.to_owned());
        }
        let native_owner = NativeThread {
            session: recorded_session.clone(),
            agent: record.source_thread.agent.clone(),
        };
        let replay_owner = message_id
            .and_then(|message_id| message_owners.get(message_id))
            .filter(|owner| {
                record.forced_copy
                    || (record.source_thread.agent.is_some() && *owner != &record.source_thread)
            })
            .or_else(|| {
                text(&record.value, &["uuid"])
                    .and_then(|uuid| uuid_owners.get(uuid))
                    .filter(|owner| *owner != &record.source_thread)
            });
        let owner = replay_owner
            .and_then(|owner| ids.get(owner))
            .or_else(|| ids.get(&native_owner))
            .or_else(|| ids.get(&NativeThread { session: recorded_session.clone(), agent: None }));
        observation.owner = owner.cloned().map_or(OwnerEvidence::None, OwnerEvidence::Proven);
        observation.role = if record.forced_copy
            || replay_owner.is_some()
            || (record.source_thread.agent.is_none()
                && recorded_session != record.source_thread.session)
        {
            ObservationRole::Copy
        } else {
            ObservationRole::Original
        };
        let (usage, model_usage) = claude_usage(&record.value)?;
        observation.usage = Some(usage);
        observation.model_usage = model_usage;
        observation.model = text(&record.value, &["message", "model"])
            .map(|name| ModelName { name: name.to_owned(), basis: ModelBasis::Served });
        observation.effort = text(&record.value, &["effort"]).map(str::to_owned);
        observation.timestamp = text(&record.value, &["timestamp"])
            .and_then(|timestamp| parse_timestamp(timestamp).ok());
        if let Some(model) = observation.model.as_ref() {
            observation.invariants.insert("model".to_owned(), model.name.clone());
        }
        observations.push(observation);
    }

    let mut ledger = reconcile(
        ReconcileInput {
            threads: threads.into_values().collect(),
            relationships,
            requests: observations,
            limit_observations,
            diagnostics,
            ..ReconcileInput::default()
        },
        &ClaudeBlockSelector,
    )?;
    for diagnostic in &mut ledger.diagnostics {
        diagnostic.code = match diagnostic.code {
            DiagnosticCode::RevisionDisagreement => DiagnosticCode::ClaudeBlockUsageConflict,
            DiagnosticCode::UsageInconsistency => {
                DiagnosticCode::ClaudeCacheCreationBreakdownMismatch
            }
            DiagnosticCode::CopyWithoutOriginal => DiagnosticCode::ClaudeNestedCopyWithoutOriginal,
            DiagnosticCode::ConflictingSharedKey => DiagnosticCode::IdentityKeyConflict,
            code @ (DiagnosticCode::ConflictingReread
            | DiagnosticCode::ConflictingOwners
            | DiagnosticCode::ConflictingAccounts
            | DiagnosticCode::ConflictingModels
            | DiagnosticCode::UnresolvedCandidate
            | DiagnosticCode::CounterReset
            | DiagnosticCode::CounterGap
            | DiagnosticCode::ClaudeBlockUsageConflict
            | DiagnosticCode::ClaudeCacheCreationBreakdownMismatch
            | DiagnosticCode::ClaudeNestedCopyWithoutOriginal
            | DiagnosticCode::IdentityKeyConflict
            | DiagnosticCode::CodexCopiedHistoryInferred
            | DiagnosticCode::CodexCounterEpochReset
            | DiagnosticCode::CodexEstimateCompaction
            | DiagnosticCode::CodexEstimateContextWindowFill
            | DiagnosticCode::CodexRolloutDuplicateLocation
            | DiagnosticCode::MalformedLine
            | DiagnosticCode::PendingTail
            | DiagnosticCode::ThreadOrphan) => code,
        };
    }
    ledger.diagnostics.sort();
    let threads = ledger.threads.clone();
    let relationships = ledger.relationships.clone();
    let limit_observations = ledger.limit_observations.clone();
    let sources = manifest
        .entries
        .iter()
        .cloned()
        .map(|snapshot| SourceArtifact {
            dialect_version: snapshot
                .source
                .as_ref()
                .and_then(|source| source_versions.get(&source.id))
                .cloned()
                .map_or(Basis::Unknown, Basis::Observed),
            capability: SourceCapability::Supported,
            snapshot,
        })
        .collect();
    Ok(Ingested { manifest, sources, threads, relationships, ledger, limit_observations })
}

fn append_limits(
    record: &ParsedRecord,
    ids: &BTreeMap<NativeThread, crate::ledger::identity::AnalyticalId>,
    ambiguous_messages: &BTreeSet<String>,
    limits: &mut Vec<ProviderLimitObservation>,
) -> Result<(), AdapterError> {
    let session =
        text(&record.value, &["sessionId"]).unwrap_or(&record.source_thread.session).to_owned();
    let owner_thread = ids
        .get(&NativeThread { session: session.clone(), agent: record.source_thread.agent.clone() })
        .cloned();
    let observed_at = text(&record.value, &["timestamp"])
        .and_then(|timestamp| parse_timestamp(timestamp).ok())
        .map_or(Basis::Unknown, Basis::Observed);
    if let Some(quota) = record.value.get("quotaLimits").and_then(Value::as_object) {
        let owner_request = text(&record.value, &["message", "id"])
            .map(|message_id| {
                if ambiguous_messages.contains(message_id) {
                    AMBIGUOUS_RESPONSE_KEY.key(vec![
                        KeyComponent::text(AGENT_NAMESPACE),
                        KeyComponent::text(format!("{session}/{message_id}")),
                    ])
                } else {
                    RESPONSE_KEY.key(vec![
                        KeyComponent::text(PROVIDER_NAMESPACE),
                        KeyComponent::text(message_id),
                    ])
                }
            })
            .transpose()?
            .map(|key| key.key.derive_id())
            .transpose()?;
        limits.push(ProviderLimitObservation {
            limit_name: quota.get("rateLimitType").and_then(Value::as_str).map(str::to_owned),
            window: None,
            observed_at: observed_at.clone(),
            owner_thread: owner_thread.clone(),
            owner_request,
            native: quota.iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
            evidence: record.evidence.clone(),
        });
    }
    if record.value.get("isApiErrorMessage").and_then(Value::as_bool) == Some(true) {
        let message = record
            .value
            .pointer("/message/content/0/text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if message.starts_with("Claude AI usage limit reached") {
            limits.push(ProviderLimitObservation {
                limit_name: None,
                window: None,
                observed_at,
                owner_thread,
                owner_request: None,
                native: BTreeMap::from([("text".to_owned(), Value::String(message.to_owned()))]),
                evidence: record.evidence.clone(),
            });
        }
    }
    Ok(())
}

fn cache_breakdown_mismatch(value: &Value) -> Option<(u64, u64)> {
    let flat = unsigned(value, &["message", "usage", "cache_creation_input_tokens"])?;
    let five =
        unsigned(value, &["message", "usage", "cache_creation", "ephemeral_5m_input_tokens"]);
    let hour =
        unsigned(value, &["message", "usage", "cache_creation", "ephemeral_1h_input_tokens"]);
    if five.is_none() && hour.is_none() {
        return None;
    }
    let breakdown = five.unwrap_or(0).checked_add(hour.unwrap_or(0))?;
    (flat != breakdown).then_some((flat, breakdown))
}

fn is_replayed_record(
    record: &ParsedRecord,
    message_owners: &BTreeMap<String, NativeThread>,
    uuid_owners: &BTreeMap<String, NativeThread>,
) -> bool {
    if record.forced_copy {
        return true;
    }
    let recorded_session =
        text(&record.value, &["sessionId"]).unwrap_or(&record.source_thread.session);
    if record.source_thread.agent.is_none() && recorded_session != record.source_thread.session {
        return true;
    }
    let message_replayed = text(&record.value, &["message", "id"])
        .and_then(|message_id| message_owners.get(message_id))
        .is_some_and(|owner| {
            record.source_thread.agent.is_some() && owner != &record.source_thread
        });
    let uuid_replayed = text(&record.value, &["uuid"])
        .and_then(|uuid| uuid_owners.get(uuid))
        .is_some_and(|owner| owner != &record.source_thread);
    message_replayed || uuid_replayed
}

fn claude_usage(value: &Value) -> Result<(TokenUsage, Vec<ModelUsage>), AdapterError> {
    let input = unsigned(value, &["message", "usage", "input_tokens"]);
    let cache_read = unsigned(value, &["message", "usage", "cache_read_input_tokens"]);
    let flat_write = unsigned(value, &["message", "usage", "cache_creation_input_tokens"]);
    let five_minute_write =
        unsigned(value, &["message", "usage", "cache_creation", "ephemeral_5m_input_tokens"]);
    let one_hour_write =
        unsigned(value, &["message", "usage", "cache_creation", "ephemeral_1h_input_tokens"]);
    let output = unsigned(value, &["message", "usage", "output_tokens"]);
    let reasoning =
        unsigned(value, &["message", "usage", "output_tokens_details", "thinking_tokens"]);
    let mut native = BTreeMap::new();
    for (name, count) in [
        ("message.usage.input_tokens", input),
        ("message.usage.cache_read_input_tokens", cache_read),
        ("message.usage.cache_creation_input_tokens", flat_write),
        ("message.usage.output_tokens", output),
    ] {
        if let Some(count) = count {
            native.insert(name.to_owned(), count);
        }
    }
    let has_breakdown = five_minute_write.is_some() || one_hour_write.is_some();
    let breakdown_total = five_minute_write.unwrap_or(0).checked_add(one_hour_write.unwrap_or(0));
    let breakdown_matches = match (flat_write, breakdown_total) {
        (Some(flat), Some(breakdown)) => flat == breakdown,
        (None, Some(_)) => true,
        (Some(_) | None, None) => false,
    };
    let measures = TokenMeasures {
        uncached_input: input,
        cache_read,
        cache_write_5m: (has_breakdown && breakdown_matches).then_some(five_minute_write).flatten(),
        cache_write_1h: (has_breakdown && breakdown_matches).then_some(one_hour_write).flatten(),
        cache_write_unspecified: (!has_breakdown || !breakdown_matches)
            .then_some(flat_write)
            .flatten(),
        output,
        reasoning,
        provider_only: None,
    };
    let primary = TokenUsage { measures, native };
    let mut usage = primary.clone();
    let mut model_usage = vec![ModelUsage {
        model: text(value, &["message", "model"])
            .map(|name| ModelName { name: name.to_owned(), basis: ModelBasis::Served }),
        usage: primary,
        source: "message.usage",
    }];
    if let Some(iterations) = value
        .get("message")
        .and_then(|message| message.get("usage"))
        .and_then(|usage| usage.get("iterations"))
        .and_then(Value::as_array)
    {
        for (index, iteration) in iterations.iter().enumerate() {
            if text(iteration, &["type"]) != Some("advisor_message") {
                continue;
            }
            let cache_write = unsigned(iteration, &["cache_creation_input_tokens"]);
            let advisor = TokenMeasures {
                uncached_input: unsigned(iteration, &["input_tokens"]),
                cache_read: unsigned(iteration, &["cache_read_input_tokens"]),
                cache_write_unspecified: cache_write,
                output: unsigned(iteration, &["output_tokens"]),
                reasoning: unsigned(iteration, &["reasoning"]),
                ..TokenMeasures::default()
            };
            usage.measures = usage.measures.checked_add(&advisor)?;
            let mut advisor_native = BTreeMap::new();
            for (field, count) in [
                ("input_tokens", advisor.uncached_input),
                ("cache_read_input_tokens", advisor.cache_read),
                ("cache_creation_input_tokens", advisor.cache_write_unspecified),
                ("output_tokens", advisor.output),
                ("reasoning", advisor.reasoning),
            ] {
                if let Some(count) = count {
                    let path = format!("message.usage.iterations[{index}].{field}");
                    usage.native.insert(path.clone(), count);
                    advisor_native.insert(path, count);
                }
            }
            model_usage.push(ModelUsage {
                model: text(iteration, &["model"])
                    .map(|name| ModelName { name: name.to_owned(), basis: ModelBasis::Served }),
                usage: TokenUsage { measures: advisor, native: advisor_native },
                source: "advisor_message",
            });
        }
    }
    Ok((usage, model_usage))
}

struct ClaudeBlockSelector;

impl RevisionSelector for ClaudeBlockSelector {
    fn rule(&self) -> &'static str {
        "claude-largest-output"
    }

    fn select(&self, revisions: &[&RequestObservation]) -> RevisionChoice {
        let selected = revisions
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| compare_claude_revision(left, right))
            .map_or(0, |(index, _)| index);
        let selected_usage = revisions[selected].usage.as_ref().map(|usage| usage.measures);
        let disagreements = selected_usage.map_or_else(Vec::new, |selected| {
            revisions
                .iter()
                .filter_map(|revision| revision.usage.as_ref())
                .any(|usage| input_measures(&usage.measures) != input_measures(&selected))
                .then(|| "input or cache fields differ across Claude block records".to_owned())
                .into_iter()
                .collect()
        });
        RevisionChoice {
            selected,
            status: crate::ledger::entities::RevisionStatus::Selected,
            disagreements,
        }
    }
}

fn input_measures(measures: &TokenMeasures) -> [Option<u64>; 6] {
    [
        measures.uncached_input,
        measures.cache_read,
        measures.cache_write_5m,
        measures.cache_write_1h,
        measures.cache_write_unspecified,
        measures.provider_only,
    ]
}

fn compare_claude_revision(left: &RequestObservation, right: &RequestObservation) -> Ordering {
    let output = |observation: &RequestObservation| {
        observation.usage.as_ref().and_then(|usage| usage.measures.output).unwrap_or(0)
    };
    output(left)
        .cmp(&output(right))
        .then_with(|| left.evidence.offset.cmp(&right.evidence.offset))
        .then_with(|| right.evidence.source.cmp(&left.evidence.source))
}

fn thread_from_path(locator: &str) -> NativeThread {
    let path = PathBuf::from(locator);
    let components: Vec<String> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let subagents = components.iter().position(|component| component == "subagents");
    match subagents {
        Some(index) if index > 0 => {
            let session = components[index.saturating_sub(1)].clone();
            let agent = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .map(|stem| stem.strip_prefix("agent-").unwrap_or(&stem).to_owned());
            NativeThread { session, agent }
        }
        _ => NativeThread {
            session: path
                .file_stem()
                .map_or_else(|| locator.to_owned(), |stem| stem.to_string_lossy().into_owned()),
            agent: None,
        },
    }
}
