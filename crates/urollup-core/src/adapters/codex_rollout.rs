//! Codex rollout adapter (`codex-rollout`).

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

use super::{AdapterError, Ingested};
use crate::ledger::counters::{CounterEvent, RunningTotal};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{
    Basis, Confidence, ModelBasis, ModelName, ProviderLimitObservation, Relationship,
    RelationshipKind, SourceArtifact, SourceCapability, Thread,
};
use crate::ledger::identity::{AnalyticalId, IdPrefix, KeyComponent, StoredIdentity};
use crate::ledger::reconcile::{
    LatestRevision, ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation, reconcile,
};
use crate::ledger::scope::{ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec};
use crate::ledger::tokens::{InputSemantics, NativeInput, TokenMeasures, normalize_input};
use crate::selection::{Agent, agent_thread_identity};
use crate::sources::decode::{parse_record, parse_timestamp, text, unsigned};
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::{ManifestEntry, SnapshotManifest};
use crate::sources::parallel::{default_workers, source_weight, try_read_in_parallel};
use crate::sources::reader::{
    RawRecord, ReadBudget, ReadOptions, RecordDisposition, SourceSpec, read_source_with_budget,
};
use crate::sources::roots::{DiscoveredSource, Discovery, discover};

const DIALECT: &str = "codex-rollout";
const PROVIDER_NAMESPACE: &str = "openai";

const PROVIDER_RESPONSE_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("provider", ComponentRole::Namespace),
    ComponentSlot::required("response_id", ComponentRole::NativeId),
];

const RESPONSE_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "provider-response",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Provider,
    slots: PROVIDER_RESPONSE_SLOTS,
};

const COUNTER_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("thread", ComponentRole::Parent),
    ComponentSlot::required("cumulative_usage", ComponentRole::Digest),
];

const COUNTER_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "thread-counter-digest",
    precedence: 1,
    basis: IdentityBasis::Fallback,
    scope: IdScope::Thread,
    slots: COUNTER_SLOTS,
};

#[derive(Clone)]
struct ParsedSource {
    file_thread: String,
    records: Vec<ParsedRecord>,
    trailing_skipped: SkippedSpan,
}

/// The accounting fields of one relevant rollout record.
///
/// Fields are extracted while the line is decoded, so no JSON document outlives its line.
#[derive(Clone)]
struct ParsedRecord {
    evidence: EvidenceRef,
    skipped_before: SkippedSpan,
    ordinal: Option<u64>,
    timestamp: Option<jiff::Timestamp>,
    kind: RecordKind,
}

/// The relevant record kinds and the fields the adapter reads from each.
#[derive(Clone)]
enum RecordKind {
    SessionMeta(Box<SessionMeta>),
    TurnContext {
        turn_id: Option<String>,
        context: TurnContext,
    },
    UsageRecord(Box<UsagePayload>),
    /// A compaction, with the latest usage record it copies when the field is present.
    Compacted(Option<Box<UsagePayload>>),
    TokenCount(Box<TokenCount>),
    ThreadSettingsApplied {
        thread_id: Option<String>,
    },
}

impl ParsedRecord {
    fn session_meta(&self) -> Option<&SessionMeta> {
        match &self.kind {
            RecordKind::SessionMeta(meta) => Some(meta),
            RecordKind::TurnContext { .. }
            | RecordKind::UsageRecord(_)
            | RecordKind::Compacted(_)
            | RecordKind::TokenCount(_)
            | RecordKind::ThreadSettingsApplied { .. } => None,
        }
    }
}

#[derive(Clone, Default)]
struct SessionMeta {
    id: Option<String>,
    cli_version: Option<String>,
    source: Option<String>,
    thread_source: Option<String>,
    /// The final component of the recorded working directory, never the path.
    project: Option<String>,
    parent_thread_id: Option<String>,
    forked_from_id: Option<String>,
    subagent_history_start_ordinal: Option<u64>,
}

impl SessionMeta {
    fn parent(&self) -> Option<&str> {
        self.parent_thread_id.as_deref().or(self.forked_from_id.as_deref())
    }
}

/// A `token_usage_record` payload, or the latest one a `compacted` record copies.
#[derive(Clone, Default)]
struct UsagePayload {
    thread_id: Option<String>,
    response_id: Option<String>,
    usage: Option<CodexUsage>,
    root_turn_id: Option<String>,
}

impl UsagePayload {
    fn from_value(payload: &Value) -> Self {
        Self {
            thread_id: text(payload, &["thread_id"]).map(str::to_owned),
            response_id: text(payload, &["response_id"]).map(str::to_owned),
            usage: payload.get("usage").map(CodexUsage::from_value),
            root_turn_id: text(payload, &["root_turn_id"]).map(str::to_owned),
        }
    }
}

#[derive(Clone)]
struct TokenCount {
    total: Option<CodexUsage>,
    last: Option<CodexUsage>,
    limits: Option<Arc<RateLimits>>,
}

/// Native Codex usage counters, each `None` when missing, null or not a count.
#[derive(Clone, Copy, Default)]
struct CodexUsage {
    input: Option<u64>,
    cached_input: Option<u64>,
    cache_write_input: Option<u64>,
    output: Option<u64>,
    reasoning_output: Option<u64>,
    total: Option<u64>,
}

impl CodexUsage {
    fn from_value(value: &Value) -> Self {
        Self {
            input: unsigned(value, &["input_tokens"]),
            cached_input: unsigned(value, &["cached_input_tokens"]),
            cache_write_input: unsigned(value, &["cache_write_input_tokens"]),
            output: unsigned(value, &["output_tokens"]),
            reasoning_output: unsigned(value, &["reasoning_output_tokens"]),
            total: unsigned(value, &["total_tokens"]),
        }
    }
}

/// One `rate_limits` object: its limit name, the windows it reports, and its fields as
/// sorted compact JSON. Consecutive identical snapshots in a rollout share one value.
struct RateLimits {
    limit_name: Option<String>,
    windows: Vec<&'static str>,
    native: String,
}

#[derive(Clone, Default)]
struct SkippedSpan {
    count: u64,
}

impl SkippedSpan {
    fn observe(&mut self) {
        self.count = self.count.saturating_add(1);
    }
}

#[derive(Clone, Debug, Default)]
struct TurnContext {
    model: Option<String>,
    effort: Option<String>,
}

/// Reads one Codex home, including active, archived and compressed rollouts.
pub fn ingest_root(root: &Path) -> Result<Ingested, AdapterError> {
    ingest_roots(&[root.to_owned()], true)
}

/// Reads Codex homes selected by discovery.
///
/// Missing variable- or flag-selected homes are errors; missing conventional defaults
/// are skipped.
pub fn ingest_roots(roots: &[PathBuf], missing_is_error: bool) -> Result<Ingested, AdapterError> {
    let rollout_roots = rollout_roots(roots);
    let discovery = discover(&rollout_roots);
    ingest_discovery(discovery, missing_is_error)
}

/// Reads a previously discovered set of Codex rollouts.
///
/// Callers that need exact session selection can filter `discovery.sources` before
/// invoking this function, avoiding a second walk and full ingestion of unrelated
/// rollouts while preserving paired plain/compressed representations. Rollouts decode on
/// the [default worker bound](default_workers) with no ingestion budget.
pub fn ingest_discovery(
    discovery: Discovery,
    missing_is_error: bool,
) -> Result<Ingested, AdapterError> {
    ingest_discovery_with_budget(
        discovery,
        missing_is_error,
        &ReadBudget::unlimited(),
        default_workers(),
    )
}

/// Reads discovered Codex rollouts on at most `workers` threads under a shared ingestion
/// budget.
///
/// Each rollout decodes independently, and the results merge in discovery order before
/// normalization, so the result is the same for every worker count. A failure returns
/// the error of the first failing rollout in discovery order, as a sequential read does.
pub fn ingest_discovery_with_budget(
    discovery: Discovery,
    missing_is_error: bool,
    budget: &ReadBudget,
    workers: NonZeroUsize,
) -> Result<Ingested, AdapterError> {
    if let (true, Some(root)) = (missing_is_error, discovery.missing_roots.first()) {
        return Err(AdapterError::MissingRoot(root.clone()));
    }
    if let Some(unreadable) = discovery.unreadable.first() {
        return Err(AdapterError::UnreadablePath {
            path: unreadable.path.clone(),
            kind: unreadable.kind,
        });
    }

    let decoded = try_read_in_parallel(
        &discovery.sources,
        workers,
        |source| source_weight(&source.files),
        |source| decode_source(source, budget),
    )?;
    let (entries, parsed_sources): (Vec<_>, Vec<_>) = decoded.into_iter().unzip();
    let manifest = SnapshotManifest { entries, skipped_links: discovery.skipped_links };

    normalize(parsed_sources, manifest)
}

/// Reads one rollout, independently of every other source.
fn decode_source(
    source: &DiscoveredSource,
    budget: &ReadBudget,
) -> Result<(ManifestEntry, ParsedSource), AdapterError> {
    let rollout = rollout_name(&source.locator);
    let stable_locator = format!("{}/{}", rollout.thread_id, rollout.rollout_id);
    let spec = SourceSpec {
        environment: "local",
        dialect: DIALECT,
        locator: &stable_locator,
        stable_locator: true,
    };
    let path = source
        .files
        .primary()
        .map_or_else(|| source.root.join(&source.locator), |(path, _)| path.to_owned());
    let mut records = Vec::new();
    let mut skipped = SkippedSpan::default();
    let mut last_limits = None;
    let entry =
        read_source_with_budget(&spec, &source.files, &ReadOptions::default(), budget, |raw| {
            decode_record(raw, &mut records, &mut skipped, &mut last_limits)
        })
        .map_err(|source| AdapterError::Read { path, source })?;
    Ok((entry, ParsedSource { file_thread: rollout.thread_id, records, trailing_skipped: skipped }))
}

fn decode_record(
    raw: &RawRecord<'_>,
    records: &mut Vec<ParsedRecord>,
    skipped: &mut SkippedSpan,
    last_limits: &mut Option<Arc<RateLimits>>,
) -> RecordDisposition {
    let Ok(value) = parse_record(raw.bytes) else {
        return RecordDisposition::Malformed;
    };
    let Some(kind) = record_kind(&value, last_limits) else {
        skipped.observe();
        return RecordDisposition::Skipped;
    };
    records.push(ParsedRecord {
        evidence: raw.evidence.clone(),
        skipped_before: std::mem::take(skipped),
        ordinal: unsigned(&value, &["ordinal"]),
        timestamp: record_timestamp(&value),
        kind,
    });
    RecordDisposition::Decoded
}

/// The accounting fields of a relevant record, or `None` for a record the adapter skips.
fn record_kind(value: &Value, last_limits: &mut Option<Arc<RateLimits>>) -> Option<RecordKind> {
    let payload = value.get("payload").unwrap_or(&Value::Null);
    match text(value, &["type"])? {
        "session_meta" => Some(RecordKind::SessionMeta(Box::new(SessionMeta {
            id: text(payload, &["id"]).map(str::to_owned),
            cli_version: text(payload, &["cli_version"]).map(str::to_owned),
            source: text(payload, &["source"]).map(str::to_owned),
            thread_source: text(payload, &["thread_source"]).map(str::to_owned),
            project: text(payload, &["cwd"])
                .and_then(|cwd| Path::new(cwd).file_name())
                .map(|name| name.to_string_lossy().into_owned()),
            parent_thread_id: text(payload, &["parent_thread_id"]).map(str::to_owned),
            forked_from_id: text(payload, &["forked_from_id"]).map(str::to_owned),
            subagent_history_start_ordinal: unsigned(payload, &["subagent_history_start_ordinal"]),
        }))),
        "turn_context" => Some(RecordKind::TurnContext {
            turn_id: text(payload, &["turn_id"]).map(str::to_owned),
            context: TurnContext {
                model: text(payload, &["model"]).map(str::to_owned),
                effort: text(payload, &["effort"]).map(str::to_owned),
            },
        }),
        "token_usage_record" => {
            Some(RecordKind::UsageRecord(Box::new(UsagePayload::from_value(payload))))
        }
        "compacted" => Some(RecordKind::Compacted(
            value
                .pointer("/payload/latest_token_usage_record")
                .map(|latest| Box::new(UsagePayload::from_value(latest))),
        )),
        "event_msg" => match text(payload, &["type"])? {
            "thread_settings_applied" => Some(RecordKind::ThreadSettingsApplied {
                thread_id: text(payload, &["thread_id"]).map(str::to_owned),
            }),
            "token_count" => Some(RecordKind::TokenCount(Box::new(TokenCount {
                total: value.pointer("/payload/info/total_token_usage").map(CodexUsage::from_value),
                last: value.pointer("/payload/info/last_token_usage").map(CodexUsage::from_value),
                limits: rate_limits(payload, last_limits),
            }))),
            _ => None,
        },
        _ => None,
    }
}

/// A token count's `rate_limits` object, sharing the previous snapshot when identical.
fn rate_limits(payload: &Value, last: &mut Option<Arc<RateLimits>>) -> Option<Arc<RateLimits>> {
    let rate_limits = payload.get("rate_limits")?;
    let object = rate_limits.as_object()?;
    let sorted: BTreeMap<&String, &Value> = object.iter().collect();
    let native = serde_json::to_string(&sorted).unwrap_or_default();
    if let Some(previous) = last.as_ref().filter(|previous| previous.native == native) {
        return Some(Arc::clone(previous));
    }
    let limits = Arc::new(RateLimits {
        limit_name: text(rate_limits, &["limit_id"])
            .or_else(|| text(rate_limits, &["limit_name"]))
            .map(str::to_owned),
        windows: ["primary", "secondary"]
            .into_iter()
            .filter(|window| object.get(*window).is_some_and(|value| !value.is_null()))
            .collect(),
        native,
    });
    *last = Some(Arc::clone(&limits));
    Some(limits)
}

/// Expands Codex homes to the rollout directories the adapter actually reads.
///
/// A root without the standard home layout is kept as-is so an explicit directory of
/// rollout files remains a valid source.
pub fn rollout_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut expanded = Vec::new();
    for root in roots {
        let candidates = [root.join("sessions"), root.join("archived_sessions")];
        let mut found_layout = false;
        for candidate in candidates {
            if candidate.is_dir() {
                expanded.push(candidate);
                found_layout = true;
            }
        }
        if !found_layout {
            expanded.push(root.clone());
        }
    }
    expanded
}

fn normalize(
    sources: Vec<ParsedSource>,
    manifest: SnapshotManifest,
) -> Result<Ingested, AdapterError> {
    let mut source_versions: BTreeMap<AnalyticalId, String> = BTreeMap::new();
    for source in &sources {
        for record in &source.records {
            if let Some(version) = record.session_meta().and_then(|meta| meta.cli_version.as_ref())
            {
                source_versions
                    .entry(record.evidence.source.clone())
                    .or_insert_with(|| version.clone());
            }
        }
    }
    let mut native_threads: BTreeSet<String> = BTreeSet::new();
    let mut meta_by_thread: BTreeMap<String, (&SessionMeta, EvidenceRef)> = BTreeMap::new();
    for source in &sources {
        native_threads.insert(source.file_thread.clone());
        for record in &source.records {
            let Some(meta) = record.session_meta() else { continue };
            if let Some(thread_id) = &meta.id {
                if *thread_id == source.file_thread {
                    meta_by_thread
                        .entry(thread_id.clone())
                        .or_insert((meta, record.evidence.clone()));
                }
            }
        }
    }

    let mut thread_ids = BTreeMap::new();
    let mut threads = BTreeMap::new();
    for native in native_threads {
        let identity = thread_identity(&native)?;
        thread_ids.insert(native.clone(), identity.id.clone());
        let meta = meta_by_thread.get(&native).map(|(meta, _)| *meta);
        let mut native_key = BTreeMap::new();
        native_key.insert("thread_id".to_owned(), native.clone());
        threads.insert(
            identity.id.clone(),
            Thread {
                identity,
                basis: IdentityBasis::Native,
                aliases: Vec::new(),
                native_key,
                source: meta
                    .and_then(|meta| meta.source.clone())
                    .map_or(Basis::Unknown, Basis::Observed),
                initiator: meta
                    .and_then(|meta| meta.thread_source.clone())
                    .map_or(Basis::Unknown, Basis::Observed),
                purpose: Basis::Unknown,
                execution_environment: Basis::Observed("local".to_owned()),
                project: meta
                    .and_then(|meta| meta.project.clone())
                    .map_or(Basis::Unknown, Basis::Inferred),
                account: Basis::Unknown,
                evidence: meta_by_thread
                    .get(&native)
                    .map(|(_, evidence)| vec![evidence.clone()])
                    .unwrap_or_default(),
            },
        );
    }

    let mut relationships = Vec::new();
    for (thread, (meta, evidence)) in &meta_by_thread {
        let relation = meta
            .parent_thread_id
            .as_deref()
            .map(|parent| (parent, RelationshipKind::Spawn))
            .or_else(|| {
                meta.forked_from_id.as_deref().map(|parent| (parent, RelationshipKind::Fork))
            });
        let Some((parent, kind)) = relation else { continue };
        let (Some(from), Some(to)) = (thread_ids.get(parent), thread_ids.get(thread)) else {
            continue;
        };
        relationships.push(Relationship {
            kind,
            from: from.clone(),
            to: to.clone(),
            confidence: Confidence::Proven,
            evidence: vec![evidence.clone()],
        });
    }

    let mut observations =
        Vec::with_capacity(sources.iter().map(|source| source.records.len()).sum());
    let mut diagnostics = source_diagnostics(&manifest);
    for (thread, (meta, evidence)) in &meta_by_thread {
        if meta.parent().is_some_and(|parent| !thread_ids.contains_key(parent)) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ThreadOrphan,
                thread_ids.get(thread).cloned(),
                [evidence.clone()],
                "Codex thread names a parent whose rollout was not discovered",
            ));
        }
    }
    let mut copied_regions = 0_u64;
    let mut limit_observations = Vec::new();
    let mut known_turns: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for source in &sources {
        let is_root =
            source.records.iter().find_map(ParsedRecord::session_meta).is_some_and(|meta| {
                meta.id.as_deref() == Some(source.file_thread.as_str())
                    && meta.parent_thread_id.is_none()
                    && meta.forked_from_id.is_none()
            });
        if is_root {
            let turns = known_turns.entry(source.file_thread.clone()).or_default();
            turns.extend(source.records.iter().filter_map(|record| match &record.kind {
                RecordKind::TurnContext { turn_id, .. } => turn_id.clone(),
                RecordKind::SessionMeta(_)
                | RecordKind::UsageRecord(_)
                | RecordKind::Compacted(_)
                | RecordKind::TokenCount(_)
                | RecordKind::ThreadSettingsApplied { .. } => None,
            }));
        }
    }
    for source in &sources {
        let has_direct =
            source.records.iter().any(|record| matches!(record.kind, RecordKind::UsageRecord(_)));
        let own_meta = source
            .records
            .iter()
            .filter_map(ParsedRecord::session_meta)
            .find(|meta| meta.id.as_deref() == Some(source.file_thread.as_str()));
        let parent_thread = own_meta.and_then(SessionMeta::parent);
        let native_boundary = own_meta.and_then(|meta| meta.subagent_history_start_ordinal);
        let has_foreign_meta = source
            .records
            .iter()
            .filter_map(ParsedRecord::session_meta)
            .any(|meta| meta.id.as_ref().is_some_and(|thread| *thread != source.file_thread));
        if parent_thread.is_some() && has_foreign_meta && (!has_direct || native_boundary.is_some())
        {
            copied_regions = copied_regions.saturating_add(1);
            if !has_direct && native_boundary.is_none() {
                let copied = legacy_copied_evidence(source, &known_turns);
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::CodexCopiedHistoryInferred,
                        thread_ids.get(&source.file_thread).cloned(),
                        copied.evidence,
                        "Codex copied-history boundary was inferred from legacy rollout records",
                    )
                    .with_occurrences(copied.occurrences),
                );
            }
        }
        let mut active_thread = source.file_thread.clone();
        let mut turns: BTreeMap<String, TurnContext> = BTreeMap::new();
        let mut current_turn = None;
        let mut last_response_by_thread: BTreeMap<String, String> = BTreeMap::new();
        let mut inherited_total = None;
        let mut counter = None;
        let mut previous_limits = BTreeMap::new();
        for record in &source.records {
            if native_boundary
                .is_some_and(|boundary| record.ordinal.is_some_and(|ordinal| ordinal >= boundary))
            {
                active_thread.clone_from(&source.file_thread);
            }
            match &record.kind {
                RecordKind::SessionMeta(meta) => {
                    if let Some(thread_id) = &meta.id {
                        active_thread.clone_from(thread_id);
                    }
                }
                RecordKind::TurnContext { turn_id, context } => {
                    if !has_direct
                        && active_thread != source.file_thread
                        && turn_id.as_ref().is_some_and(|turn| {
                            known_turns
                                .get(&active_thread)
                                .is_some_and(|known| !known.contains(turn))
                        })
                    {
                        active_thread.clone_from(&source.file_thread);
                    }
                    if let Some(turn_id) = turn_id {
                        turns.insert(turn_id.clone(), context.clone());
                    }
                    current_turn.clone_from(turn_id);
                }
                RecordKind::ThreadSettingsApplied { thread_id } => {
                    active_thread.clone_from(thread_id.as_ref().unwrap_or(&source.file_thread));
                }
                RecordKind::UsageRecord(payload) => {
                    let observation = direct_observation(
                        record,
                        payload,
                        &source.file_thread,
                        &thread_ids,
                        &turns,
                    )?;
                    if let Some(response_id) = payload.response_id.clone() {
                        let owner = payload.thread_id.as_ref().unwrap_or(&source.file_thread);
                        last_response_by_thread.insert(owner.clone(), response_id);
                    }
                    observations.push(observation);
                }
                RecordKind::Compacted(latest) => {
                    if let Some(latest) = latest {
                        observations.push(usage_observation(
                            record,
                            latest,
                            ObservationRole::Copy,
                            &source.file_thread,
                            &thread_ids,
                            &turns,
                        )?);
                    }
                }
                RecordKind::TokenCount(count) => {
                    if let Some(limits) = &count.limits {
                        append_limits(
                            record,
                            limits,
                            &active_thread,
                            &thread_ids,
                            &mut previous_limits,
                            &mut limit_observations,
                        );
                    }
                    if has_direct && active_thread != source.file_thread {
                        if let Some(usage) = &count.last {
                            let mut observation =
                                RequestObservation::new(record.evidence.clone(), DIALECT);
                            observation.role = ObservationRole::Copy;
                            observation.owner = thread_ids
                                .get(&active_thread)
                                .cloned()
                                .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
                            observation.usage = Some(codex_usage(usage)?);
                            if let Some(response_id) = last_response_by_thread.get(&active_thread) {
                                observation.keys.push(
                                    RESPONSE_KEY
                                        .key(vec![
                                            KeyComponent::text(PROVIDER_NAMESPACE),
                                            KeyComponent::text(response_id),
                                        ])?
                                        .derive()?,
                                );
                            }
                            observation.timestamp = record.timestamp;
                            if let Some(context) =
                                current_turn.as_ref().and_then(|turn| turns.get(turn))
                            {
                                apply_context(&mut observation, context);
                            }
                            observations.push(observation);
                        }
                    } else if !has_direct {
                        let Some(total) = &count.total else { continue };
                        let total_usage = codex_usage(total)?;
                        let last = count.last.as_ref();
                        if active_thread != source.file_thread {
                            inherited_total = Some(total_usage);
                            if let Some(last) = last {
                                observations.push(counter_observation(
                                    record,
                                    last,
                                    total,
                                    CounterObservation {
                                        role: ObservationRole::Copy,
                                        owner: &active_thread,
                                        thread_ids: &thread_ids,
                                        context: current_turn
                                            .as_ref()
                                            .and_then(|turn| turns.get(turn)),
                                        delta: None,
                                    },
                                )?);
                            }
                            continue;
                        }

                        let tracker = counter.get_or_insert_with(|| {
                            inherited_total.map_or_else(RunningTotal::new, RunningTotal::inheriting)
                        });
                        let step = tracker.observe(&total_usage, None)?;
                        if step.event == CounterEvent::Reset {
                            diagnostics.push(Diagnostic::new(
                                DiagnosticCode::CodexCounterEpochReset,
                                thread_ids.get(&source.file_thread).cloned(),
                                [record.evidence.clone()],
                                "Codex cumulative usage decreased and opened a new counter epoch",
                            ));
                        }
                        let Some(last) = last else { continue };
                        let estimated = last.input.unwrap_or(0) == 0
                            && last.output.unwrap_or(0) == 0
                            && last.total.unwrap_or(0) > 0;
                        if estimated {
                            let context_fill = total.input.unwrap_or(0) == 0
                                && total.output.unwrap_or(0) == 0
                                && total.total.unwrap_or(0) > 0;
                            diagnostics.push(Diagnostic::new(
                                if context_fill {
                                    DiagnosticCode::CodexEstimateContextWindowFill
                                } else {
                                    DiagnosticCode::CodexEstimateCompaction
                                },
                                thread_ids.get(&source.file_thread).cloned(),
                                [record.evidence.clone()],
                                "Codex emitted an estimate with zero input and output tokens",
                            ));
                            continue;
                        }
                        if step.event != CounterEvent::Repeated {
                            observations.push(counter_observation(
                                record,
                                last,
                                total,
                                CounterObservation {
                                    role: ObservationRole::Original,
                                    owner: &source.file_thread,
                                    thread_ids: &thread_ids,
                                    context: current_turn.as_ref().and_then(|turn| turns.get(turn)),
                                    delta: Some(step.delta),
                                },
                            )?);
                        }
                    }
                }
            }
        }
    }

    // Decoded records are not needed once observations exist; free them before the
    // reconciliation peak.
    drop(sources);
    let mut ledger = reconcile(
        ReconcileInput {
            threads: threads.into_values().collect(),
            relationships,
            requests: observations,
            limit_observations,
            diagnostics,
            ..ReconcileInput::default()
        },
        &LatestRevision,
    )?;
    ledger.coverage.copies = ledger.coverage.copies.saturating_add(copied_regions);
    ledger.diagnostics.retain(|diagnostic| diagnostic.code != DiagnosticCode::CopyWithoutOriginal);
    let threads = std::mem::take(&mut ledger.threads);
    let relationships = std::mem::take(&mut ledger.relationships);
    let limit_observations = std::mem::take(&mut ledger.limit_observations);
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

fn legacy_copied_evidence(
    source: &ParsedSource,
    known_turns: &BTreeMap<String, BTreeSet<String>>,
) -> CopiedEvidence {
    let mut active_thread = source.file_thread.clone();
    let mut evidence = Vec::new();
    let mut occurrences = 0_u64;
    for record in &source.records {
        if active_thread != source.file_thread {
            occurrences = occurrences.saturating_add(record.skipped_before.count);
        }
        match &record.kind {
            RecordKind::SessionMeta(meta) => {
                if let Some(thread_id) = &meta.id {
                    active_thread.clone_from(thread_id);
                }
            }
            RecordKind::TurnContext { turn_id, .. } if active_thread != source.file_thread => {
                let is_child_turn = turn_id.as_ref().is_some_and(|turn| {
                    known_turns.get(&active_thread).is_some_and(|known| !known.contains(turn))
                });
                if is_child_turn {
                    active_thread.clone_from(&source.file_thread);
                }
            }
            RecordKind::ThreadSettingsApplied { .. } => {
                active_thread.clone_from(&source.file_thread);
            }
            RecordKind::TurnContext { .. }
            | RecordKind::UsageRecord(_)
            | RecordKind::Compacted(_)
            | RecordKind::TokenCount(_) => {}
        }
        if active_thread != source.file_thread {
            occurrences = occurrences.saturating_add(1);
            evidence.push(record.evidence.clone());
        }
    }
    if active_thread != source.file_thread {
        occurrences = occurrences.saturating_add(source.trailing_skipped.count);
    }
    CopiedEvidence { evidence, occurrences }
}

struct CopiedEvidence {
    evidence: Vec<EvidenceRef>,
    occurrences: u64,
}

#[derive(Clone, Copy)]
struct CounterObservation<'a> {
    role: ObservationRole,
    owner: &'a str,
    thread_ids: &'a BTreeMap<String, AnalyticalId>,
    context: Option<&'a TurnContext>,
    delta: Option<TokenMeasures>,
}

fn counter_observation(
    record: &ParsedRecord,
    last: &CodexUsage,
    total: &CodexUsage,
    counter: CounterObservation<'_>,
) -> Result<RequestObservation, AdapterError> {
    let mut observation = RequestObservation::new(record.evidence.clone(), DIALECT);
    observation.role = counter.role;
    observation.owner = counter
        .thread_ids
        .get(counter.owner)
        .cloned()
        .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    if let Some(thread_id) = counter.thread_ids.get(counter.owner) {
        observation.keys.push(
            COUNTER_KEY
                .key(vec![
                    KeyComponent::text(thread_id.to_string()),
                    KeyComponent::text(counter_signature(total)),
                ])?
                .derive()?,
        );
    }
    let mut usage = codex_usage(last)?;
    if let Some(delta) = counter.delta {
        usage = delta;
    }
    observation.usage = Some(usage);
    observation.timestamp = record.timestamp;
    if let Some(context) = counter.context {
        apply_context(&mut observation, context);
    }
    Ok(observation)
}

/// Emits one limit observation per reported window, skipping a snapshot identical to the
/// previous one in the same stream of this rollout: owner thread, limit and window.
/// The stream a limit observation collapses within: owner thread, limit name and window.
type LimitStream = (Option<AnalyticalId>, Option<String>, &'static str);

fn append_limits(
    record: &ParsedRecord,
    limits: &Arc<RateLimits>,
    owner: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    previous: &mut BTreeMap<LimitStream, Arc<RateLimits>>,
    observations: &mut Vec<ProviderLimitObservation>,
) {
    let owner_thread = thread_ids.get(owner).cloned();
    for window in &limits.windows {
        let stream = (owner_thread.clone(), limits.limit_name.clone(), *window);
        let repeated = previous.get(&stream).is_some_and(|last| last.native == limits.native);
        previous.insert(stream, Arc::clone(limits));
        if repeated {
            continue;
        }
        observations.push(ProviderLimitObservation {
            limit_name: limits.limit_name.clone(),
            window: Some((*window).to_owned()),
            observed_at: record.timestamp.map_or(Basis::Unknown, Basis::Observed),
            owner_thread: owner_thread.clone(),
            owner_request: None,
            native: limits.native.as_str().into(),
            evidence: record.evidence.clone(),
        });
    }
}

fn source_diagnostics(manifest: &SnapshotManifest) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut locations: BTreeMap<&str, Vec<&crate::sources::manifest::ManifestEntry>> =
        BTreeMap::new();
    for entry in &manifest.entries {
        locations.entry(&entry.locator).or_default().push(entry);
        if let Some(evidence) = &entry.first_malformed {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::MalformedLine,
                None,
                [evidence.clone()],
                "a complete Codex rollout line is malformed",
            ));
        }
        if let (Some(source), Some(tail)) = (&entry.source, entry.cutoff.pending_tail) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::PendingTail,
                None,
                [EvidenceRef {
                    source: source.id.clone(),
                    offset: tail.offset,
                    length: tail.length,
                }],
                "the incomplete final Codex rollout line is pending",
            ));
        }
    }
    for entries in locations.values().filter(|entries| entries.len() > 1) {
        let evidence = entries.iter().filter_map(|entry| {
            entry.source.as_ref().map(|source| EvidenceRef {
                source: source.id.clone(),
                offset: 0,
                length: 0,
            })
        });
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::CodexRolloutDuplicateLocation,
                None,
                evidence,
                "the same Codex thread and rollout were found at multiple locations",
            )
            .with_occurrences(u64::try_from(entries.len()).unwrap_or(u64::MAX)),
        );
    }
    diagnostics
}

fn counter_signature(total: &CodexUsage) -> String {
    [
        total.input,
        total.cached_input,
        total.cache_write_input,
        total.output,
        total.reasoning_output,
        total.total,
    ]
    .map(|count| count.map_or_else(|| "?".to_owned(), |value| value.to_string()))
    .join(":")
}

fn direct_observation(
    record: &ParsedRecord,
    payload: &UsagePayload,
    file_thread: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    turns: &BTreeMap<String, TurnContext>,
) -> Result<RequestObservation, AdapterError> {
    usage_observation(record, payload, ObservationRole::Original, file_thread, thread_ids, turns)
}

fn usage_observation(
    record: &ParsedRecord,
    payload: &UsagePayload,
    default_role: ObservationRole,
    file_thread: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    turns: &BTreeMap<String, TurnContext>,
) -> Result<RequestObservation, AdapterError> {
    let owner = payload.thread_id.as_deref().unwrap_or(file_thread);
    let mut observation = RequestObservation::new(record.evidence.clone(), DIALECT);
    observation.role = if default_role == ObservationRole::Copy || owner != file_thread {
        ObservationRole::Copy
    } else {
        ObservationRole::Original
    };
    observation.owner =
        thread_ids.get(owner).cloned().map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    if let Some(response_id) = &payload.response_id {
        observation.keys.push(
            RESPONSE_KEY
                .key(vec![KeyComponent::text(PROVIDER_NAMESPACE), KeyComponent::text(response_id)])?
                .derive()?,
        );
    }
    if let Some(usage) = &payload.usage {
        observation.usage = Some(codex_usage(usage)?);
    }
    observation.timestamp = record.timestamp;
    if let Some(context) = payload.root_turn_id.as_ref().and_then(|turn| turns.get(turn)) {
        apply_context(&mut observation, context);
    }
    Ok(observation)
}

fn apply_context(observation: &mut RequestObservation, context: &TurnContext) {
    observation.model = context
        .model
        .as_ref()
        .map(|name| ModelName { name: name.clone(), basis: ModelBasis::Requested });
    observation.effort.clone_from(&context.effort);
}

fn codex_usage(usage: &CodexUsage) -> Result<TokenMeasures, AdapterError> {
    let mut measures = normalize_input(
        InputSemantics::IncludesCacheRead,
        NativeInput {
            input: usage.input,
            cache_read: usage.cached_input,
            cache_write: usage.cache_write_input,
        },
    )?;
    measures.output = usage.output;
    measures.reasoning = usage.reasoning_output;
    Ok(measures)
}

fn record_timestamp(value: &Value) -> Option<jiff::Timestamp> {
    text(value, &["timestamp"]).and_then(|timestamp| parse_timestamp(timestamp).ok())
}

fn thread_identity(native: &str) -> Result<StoredIdentity, AdapterError> {
    Ok(agent_thread_identity(Agent::Codex, native)?)
}

struct RolloutName {
    thread_id: String,
    rollout_id: String,
}

fn rollout_name(locator: &str) -> RolloutName {
    let name = PathBuf::from(locator)
        .file_name()
        .map_or_else(|| locator.to_owned(), |name| name.to_string_lossy().into_owned());
    let stem =
        name.strip_suffix(".jsonl.zst").or_else(|| name.strip_suffix(".jsonl")).unwrap_or(&name);
    if let Some((base, rollout)) = stem.rsplit_once('_') {
        let thread = base.get(base.len().saturating_sub(36)..).unwrap_or(base);
        return RolloutName { thread_id: thread.to_owned(), rollout_id: rollout.to_owned() };
    }
    let thread = stem.get(stem.len().saturating_sub(36)..).unwrap_or(stem).to_owned();
    RolloutName { rollout_id: thread.clone(), thread_id: thread }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use super::{
        RateLimits, RecordKind, SkippedSpan, decode_record, ingest_discovery_with_budget,
        ingest_root, record_kind,
    };
    use crate::adapters::AdapterError;
    use crate::ledger::diagnostics::DiagnosticCode;
    use crate::ledger::identity::AnalyticalId;
    use crate::sources::evidence::EvidenceRef;
    use crate::sources::reader::{RawRecord, ReadBudget, RecordDisposition, SourceReadError};
    use crate::sources::roots::discover;

    #[test]
    fn skipped_record_payloads_are_not_retained() {
        let source = AnalyticalId::parse("src-v1-00000000000000000000000000").unwrap();
        let payload = "x".repeat(2 * 1024 * 1024);
        let bytes = format!(r#"{{"type":"response_item","payload":{{"content":"{payload}"}}}}"#);
        let evidence =
            EvidenceRef { source, offset: 0, length: u64::try_from(bytes.len()).unwrap() };
        let raw = RawRecord { evidence: &evidence, bytes: bytes.as_bytes() };
        let mut records = Vec::new();
        let mut skipped = SkippedSpan::default();

        assert_eq!(
            decode_record(&raw, &mut records, &mut skipped, &mut None),
            RecordDisposition::Skipped
        );
        assert!(records.is_empty());
        assert_eq!(skipped.count, 1);
    }

    #[test]
    fn legacy_copied_history_counts_skipped_spans_without_retaining_their_evidence() {
        let home = tempfile::tempdir().unwrap();
        let sessions = home.path().join("sessions/2026/09/16");
        fs::create_dir_all(&sessions).unwrap();
        let parent = "00000000-0000-7000-8000-000000000001";
        let child = "00000000-0000-7000-8000-000000000002";
        let rollout = sessions.join(format!("rollout-2026-09-16T12-00-00-{child}.jsonl"));
        fs::write(
            rollout,
            format!(
                concat!(
                    "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{child}\",\"parent_thread_id\":\"{parent}\"}}}}\n",
                    "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{parent}\"}}}}\n",
                    "{{\"type\":\"response_item\",\"payload\":{{\"content\":\"inside copied history\"}}}}\n",
                    "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\"}}}}\n",
                    "{{\"type\":\"response_item\",\"payload\":{{\"content\":\"trailing copied history\"}}}}\n"
                ),
                child = child,
                parent = parent,
            ),
        )
        .unwrap();

        let ingested = ingest_root(home.path()).unwrap();
        let copied = ingested
            .ledger
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagnosticCode::CodexCopiedHistoryInferred)
            .expect("legacy copied history emits a diagnostic");

        assert_eq!(copied.occurrences, 4);
        assert_eq!(copied.evidence.len(), 2, "only relevant copied records retain evidence");
        assert_eq!(ingested.manifest.entries[0].counters.skipped, 2);
    }

    #[test]
    fn codex_home_discovers_only_rollout_directories() {
        let home = tempfile::tempdir().unwrap();
        let sessions = home.path().join("sessions/2026/09/16");
        fs::create_dir_all(&sessions).unwrap();
        let thread = "00000000-0000-7000-8000-000000000001";
        let rollout = sessions.join(format!("rollout-2026-09-16T12-00-00-{thread}.jsonl"));
        fs::write(rollout, format!(r#"{{"type":"session_meta","payload":{{"id":"{thread}"}}}}"#))
            .unwrap();
        fs::write(
            home.path().join("unrelated.jsonl"),
            r#"{"type":"response_item","payload":{"content":"not a rollout"}}"#,
        )
        .unwrap();

        let ingested = ingest_root(home.path()).unwrap();

        assert_eq!(ingested.manifest.entries.len(), 1);
    }

    #[test]
    fn adapter_threads_one_record_budget_across_rollout_files() {
        let home = tempfile::tempdir().unwrap();
        let sessions = home.path().join("sessions/2026/09/16");
        fs::create_dir_all(&sessions).unwrap();
        for suffix in ["000000000001", "000000000002"] {
            let thread = format!("00000000-0000-7000-8000-{suffix}");
            let rollout = sessions.join(format!("rollout-2026-09-16T12-00-00-{thread}.jsonl"));
            fs::write(
                rollout,
                format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread}\"}}}}\n"),
            )
            .unwrap();
        }
        for workers in [1, 8] {
            let discovery = discover(std::slice::from_ref(&sessions));
            let budget = ReadBudget::new(u64::MAX, 1);
            let workers = NonZeroUsize::new(workers).unwrap();

            let error =
                ingest_discovery_with_budget(discovery, true, &budget, workers).unwrap_err();

            assert!(
                matches!(
                    error,
                    AdapterError::Read {
                        source: SourceReadError::RecordBudgetExceeded { maximum: 1 },
                        ..
                    }
                ),
                "{workers} workers"
            );
        }
    }

    #[test]
    fn relevant_records_keep_only_accounting_fields() {
        let payload = "x".repeat(2 * 1024 * 1024);
        let value = serde_json::json!({
            "type": "compacted",
            "timestamp": "2026-09-16T12:00:00Z",
            "payload": {
                "latest_token_usage_record": {
                    "response_id": "response-one",
                    "usage": {"input_tokens": 3, "output_tokens": 10}
                },
                "replacement_history": [{"content": payload}]
            }
        });

        let latest = match record_kind(&value, &mut None) {
            Some(RecordKind::Compacted(latest)) => latest,
            _ => None,
        }
        .expect("a compacted record keeps its latest usage record");

        assert_eq!(latest.response_id.as_deref(), Some("response-one"));
        assert_eq!(latest.usage.and_then(|usage| usage.input), Some(3));
        assert_eq!(latest.usage.and_then(|usage| usage.output), Some(10));
    }

    #[test]
    fn identical_consecutive_rate_limits_share_one_snapshot() {
        let token_count = |used: f64| {
            serde_json::json!({
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "rate_limits": {"limit_id": "codex", "primary": {"used_percent": used}}
                }
            })
        };
        let limits = |value: &serde_json::Value, last: &mut Option<Arc<RateLimits>>| {
            match record_kind(value, last) {
                Some(RecordKind::TokenCount(count)) => count.limits,
                _ => None,
            }
            .expect("a token count with rate limits keeps them")
        };
        let mut last = None;
        let first = limits(&token_count(1.0), &mut last);
        let repeat = limits(&token_count(1.0), &mut last);
        let changed = limits(&token_count(2.0), &mut last);

        assert!(Arc::ptr_eq(&first, &repeat));
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(first.windows, ["primary"]);
        assert_eq!(first.limit_name.as_deref(), Some("codex"));
    }
}
