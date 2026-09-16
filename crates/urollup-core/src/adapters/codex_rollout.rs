//! Codex rollout adapter (`codex-rollout`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{AdapterError, Ingested};
use crate::ledger::counters::{CounterEvent, RunningTotal};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{
    Basis, Confidence, ModelBasis, ModelName, ModelUsage, ProviderLimitObservation, Relationship,
    RelationshipKind, SourceArtifact, SourceCapability, Thread,
};
use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent, StoredIdentity};
use crate::ledger::reconcile::{
    LatestRevision, ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation, reconcile,
};
use crate::ledger::scope::{ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec};
use crate::ledger::tokens::{InputSemantics, NativeInput, TokenUsage, normalize_input};
use crate::sources::decode::{parse_record, parse_timestamp, text, unsigned};
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::SnapshotManifest;
use crate::sources::reader::{RawRecord, ReadOptions, RecordDisposition, SourceSpec, read_source};
use crate::sources::roots::discover;

const DIALECT: &str = "codex-rollout";
const AGENT_NAMESPACE: &str = "codex";
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
}

#[derive(Clone)]
struct ParsedRecord {
    evidence: EvidenceRef,
    value: Value,
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

    let mut parsed_sources = Vec::new();
    let mut manifest =
        SnapshotManifest { entries: Vec::new(), skipped_links: discovery.skipped_links };
    for source in discovery.sources {
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
        let entry = read_source(&spec, &source.files, &ReadOptions::default(), |raw| {
            decode_record(raw, &mut records)
        })
        .map_err(|source| AdapterError::Read { path, source })?;
        manifest.entries.push(entry);
        parsed_sources.push(ParsedSource { file_thread: rollout.thread_id, records });
    }

    normalize(&parsed_sources, manifest)
}

fn decode_record(raw: &RawRecord<'_>, records: &mut Vec<ParsedRecord>) -> RecordDisposition {
    let Ok(value) = parse_record(raw.bytes) else {
        return RecordDisposition::Malformed;
    };
    let relevant = matches!(
        text(&value, &["type"]),
        Some("session_meta" | "turn_context" | "token_usage_record" | "compacted")
    ) || (text(&value, &["type"]) == Some("event_msg")
        && matches!(
            text(&value, &["payload", "type"]),
            Some("thread_settings_applied" | "token_count")
        ));
    records.push(ParsedRecord { evidence: raw.evidence.clone(), value });
    if relevant { RecordDisposition::Decoded } else { RecordDisposition::Skipped }
}

fn normalize(
    sources: &[ParsedSource],
    manifest: SnapshotManifest,
) -> Result<Ingested, AdapterError> {
    let mut source_versions: BTreeMap<AnalyticalId, String> = BTreeMap::new();
    for source in sources {
        for record in &source.records {
            if let Some(version) = text(&record.value, &["payload", "cli_version"]) {
                source_versions
                    .entry(record.evidence.source.clone())
                    .or_insert_with(|| version.to_owned());
            }
        }
    }
    let mut native_threads: BTreeSet<String> = BTreeSet::new();
    let mut meta_by_thread: BTreeMap<String, (&Value, EvidenceRef)> = BTreeMap::new();
    for source in sources {
        native_threads.insert(source.file_thread.clone());
        for record in &source.records {
            if text(&record.value, &["type"]) != Some("session_meta") {
                continue;
            }
            if let Some(thread_id) = text(&record.value, &["payload", "id"]) {
                if thread_id == source.file_thread {
                    meta_by_thread
                        .entry(thread_id.to_owned())
                        .or_insert((&record.value, record.evidence.clone()));
                }
            }
        }
    }

    let mut thread_ids = BTreeMap::new();
    let mut threads = BTreeMap::new();
    for native in native_threads {
        let identity = thread_identity(&native)?;
        thread_ids.insert(native.clone(), identity.id.clone());
        let meta = meta_by_thread.get(&native).map(|(value, _)| *value);
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
                    .and_then(|value| text(value, &["payload", "source"]))
                    .map_or(Basis::Unknown, |value| Basis::Observed(value.to_owned())),
                initiator: meta
                    .and_then(|value| text(value, &["payload", "thread_source"]))
                    .map_or(Basis::Unknown, |value| Basis::Observed(value.to_owned())),
                purpose: Basis::Unknown,
                execution_environment: Basis::Observed("local".to_owned()),
                project: meta
                    .and_then(|value| text(value, &["payload", "cwd"]))
                    .and_then(|cwd| Path::new(cwd).file_name())
                    .map_or(Basis::Unknown, |name| {
                        Basis::Inferred(name.to_string_lossy().into_owned())
                    }),
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
        let relation = text(meta, &["payload", "parent_thread_id"])
            .map(|parent| (parent, RelationshipKind::Spawn))
            .or_else(|| {
                text(meta, &["payload", "forked_from_id"])
                    .map(|parent| (parent, RelationshipKind::Fork))
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

    let mut observations = Vec::new();
    let mut diagnostics = source_diagnostics(&manifest);
    for (thread, (meta, evidence)) in &meta_by_thread {
        let parent = text(meta, &["payload", "parent_thread_id"])
            .or_else(|| text(meta, &["payload", "forked_from_id"]));
        if parent.is_some_and(|parent| !thread_ids.contains_key(parent)) {
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
    for source in sources {
        let is_root = source
            .records
            .iter()
            .find(|record| text(&record.value, &["type"]) == Some("session_meta"))
            .is_some_and(|record| {
                text(&record.value, &["payload", "id"]) == Some(&source.file_thread)
                    && text(&record.value, &["payload", "parent_thread_id"]).is_none()
                    && text(&record.value, &["payload", "forked_from_id"]).is_none()
            });
        if is_root {
            let turns = known_turns.entry(source.file_thread.clone()).or_default();
            turns.extend(source.records.iter().filter_map(|record| {
                (text(&record.value, &["type"]) == Some("turn_context"))
                    .then(|| text(&record.value, &["payload", "turn_id"]).map(str::to_owned))
                    .flatten()
            }));
        }
    }
    for source in sources {
        let has_direct = source
            .records
            .iter()
            .any(|record| text(&record.value, &["type"]) == Some("token_usage_record"));
        let own_meta = source.records.iter().find(|record| {
            text(&record.value, &["type"]) == Some("session_meta")
                && text(&record.value, &["payload", "id"]) == Some(&source.file_thread)
        });
        let parent_thread = own_meta.and_then(|record| {
            text(&record.value, &["payload", "parent_thread_id"])
                .or_else(|| text(&record.value, &["payload", "forked_from_id"]))
        });
        let native_boundary = own_meta.and_then(|record| {
            unsigned(&record.value, &["payload", "subagent_history_start_ordinal"])
        });
        let has_foreign_meta = source.records.iter().any(|record| {
            text(&record.value, &["type"]) == Some("session_meta")
                && text(&record.value, &["payload", "id"])
                    .is_some_and(|thread| thread != source.file_thread)
        });
        if parent_thread.is_some() && has_foreign_meta && (!has_direct || native_boundary.is_some())
        {
            copied_regions = copied_regions.saturating_add(1);
            if !has_direct && native_boundary.is_none() {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::CodexCopiedHistoryInferred,
                    thread_ids.get(&source.file_thread).cloned(),
                    legacy_copied_evidence(source, &known_turns),
                    "Codex copied-history boundary was inferred from legacy rollout records",
                ));
            }
        }
        let mut active_thread = source.file_thread.clone();
        let mut turns: BTreeMap<String, TurnContext> = BTreeMap::new();
        let mut current_turn = None;
        let mut last_response_by_thread: BTreeMap<String, String> = BTreeMap::new();
        let mut inherited_total = None;
        let mut counter = None;
        for record in &source.records {
            if native_boundary.is_some_and(|boundary| {
                unsigned(&record.value, &["ordinal"]).is_some_and(|ordinal| ordinal >= boundary)
            }) {
                active_thread.clone_from(&source.file_thread);
            }
            if text(&record.value, &["type"]) == Some("event_msg")
                && text(&record.value, &["payload", "type"]) == Some("token_count")
            {
                append_limits(record, &active_thread, &thread_ids, &mut limit_observations);
            }
            match text(&record.value, &["type"]) {
                Some("session_meta") => {
                    if let Some(thread_id) = text(&record.value, &["payload", "id"]) {
                        thread_id.clone_into(&mut active_thread);
                    }
                }
                Some("turn_context") => {
                    let turn_id = text(&record.value, &["payload", "turn_id"]).map(str::to_owned);
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
                    let context = TurnContext {
                        model: text(&record.value, &["payload", "model"]).map(str::to_owned),
                        effort: text(&record.value, &["payload", "effort"]).map(str::to_owned),
                    };
                    if let Some(turn_id) = &turn_id {
                        turns.insert(turn_id.clone(), context);
                    }
                    current_turn = turn_id;
                }
                Some("event_msg")
                    if text(&record.value, &["payload", "type"])
                        == Some("thread_settings_applied") =>
                {
                    text(&record.value, &["payload", "thread_id"])
                        .unwrap_or(&source.file_thread)
                        .clone_into(&mut active_thread);
                }
                Some("token_usage_record") => {
                    let observation =
                        direct_observation(record, &source.file_thread, &thread_ids, &turns)?;
                    if let Some(response_id) = observation.native_response_id.clone() {
                        let owner = text(&record.value, &["payload", "thread_id"])
                            .unwrap_or(&source.file_thread);
                        last_response_by_thread.insert(owner.to_owned(), response_id);
                    }
                    observations.push(observation);
                }
                Some("compacted") => {
                    if let Some(usage) = record.value.pointer("/payload/latest_token_usage_record")
                    {
                        observations.push(usage_observation(
                            record,
                            usage,
                            ObservationRole::Copy,
                            &source.file_thread,
                            &thread_ids,
                            &turns,
                        )?);
                    }
                }
                Some("event_msg")
                    if has_direct
                        && active_thread != source.file_thread
                        && text(&record.value, &["payload", "type"]) == Some("token_count") =>
                {
                    if let Some(usage) = record.value.pointer("/payload/info/last_token_usage") {
                        let mut observation =
                            RequestObservation::new(record.evidence.clone(), DIALECT);
                        observation.role = ObservationRole::Copy;
                        observation.owner = thread_ids
                            .get(&active_thread)
                            .cloned()
                            .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
                        observation.usage = Some(codex_usage(usage)?);
                        if let Some(response_id) = last_response_by_thread.get(&active_thread) {
                            observation.keys.push(RESPONSE_KEY.key(vec![
                                KeyComponent::text(PROVIDER_NAMESPACE),
                                KeyComponent::text(response_id),
                            ])?);
                            observation.native_response_id = Some(response_id.clone());
                        }
                        observation.timestamp = record_timestamp(&record.value);
                        if let Some(context) =
                            current_turn.as_ref().and_then(|turn| turns.get(turn))
                        {
                            apply_context(&mut observation, context);
                        }
                        set_model_usage(&mut observation, "token_count.last_token_usage");
                        observations.push(observation);
                    }
                }
                Some("event_msg")
                    if !has_direct
                        && text(&record.value, &["payload", "type"]) == Some("token_count") =>
                {
                    let Some(total) = record.value.pointer("/payload/info/total_token_usage")
                    else {
                        continue;
                    };
                    let total_usage = codex_usage(total)?;
                    let last = record.value.pointer("/payload/info/last_token_usage");
                    if active_thread != source.file_thread {
                        inherited_total = Some(total_usage.measures);
                        if let Some(last) = last {
                            observations.push(counter_observation(
                                record,
                                last,
                                total,
                                CounterObservation {
                                    role: ObservationRole::Copy,
                                    owner: &active_thread,
                                    thread_ids: &thread_ids,
                                    context: current_turn.as_ref().and_then(|turn| turns.get(turn)),
                                    delta: None,
                                },
                            )?);
                        }
                        continue;
                    }

                    let tracker = counter.get_or_insert_with(|| {
                        inherited_total.map_or_else(RunningTotal::new, RunningTotal::inheriting)
                    });
                    let step = tracker.observe(&total_usage.measures, None)?;
                    if step.event == CounterEvent::Reset {
                        diagnostics.push(Diagnostic::new(
                            DiagnosticCode::CodexCounterEpochReset,
                            thread_ids.get(&source.file_thread).cloned(),
                            [record.evidence.clone()],
                            "Codex cumulative usage decreased and opened a new counter epoch",
                        ));
                    }
                    let Some(last) = last else { continue };
                    let estimated = unsigned(last, &["input_tokens"]).unwrap_or(0) == 0
                        && unsigned(last, &["output_tokens"]).unwrap_or(0) == 0
                        && unsigned(last, &["total_tokens"]).unwrap_or(0) > 0;
                    if estimated {
                        let context_fill = unsigned(total, &["input_tokens"]).unwrap_or(0) == 0
                            && unsigned(total, &["output_tokens"]).unwrap_or(0) == 0
                            && unsigned(total, &["total_tokens"]).unwrap_or(0) > 0;
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
                Some(_) | None => {}
            }
        }
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
        &LatestRevision,
    )?;
    ledger.coverage.copies = ledger.coverage.copies.saturating_add(copied_regions);
    ledger.diagnostics.retain(|diagnostic| diagnostic.code != DiagnosticCode::CopyWithoutOriginal);
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

fn legacy_copied_evidence(
    source: &ParsedSource,
    known_turns: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<EvidenceRef> {
    let mut active_thread = source.file_thread.clone();
    let mut evidence = Vec::new();
    for record in &source.records {
        match text(&record.value, &["type"]) {
            Some("session_meta") => {
                if let Some(thread_id) = text(&record.value, &["payload", "id"]) {
                    thread_id.clone_into(&mut active_thread);
                }
            }
            Some("turn_context") if active_thread != source.file_thread => {
                let is_child_turn =
                    text(&record.value, &["payload", "turn_id"]).is_some_and(|turn| {
                        known_turns.get(&active_thread).is_some_and(|known| !known.contains(turn))
                    });
                if is_child_turn {
                    active_thread.clone_from(&source.file_thread);
                }
            }
            Some("event_msg")
                if text(&record.value, &["payload", "type"]) == Some("thread_settings_applied") =>
            {
                active_thread.clone_from(&source.file_thread);
            }
            Some(_) | None => {}
        }
        if active_thread != source.file_thread {
            evidence.push(record.evidence.clone());
        }
    }
    evidence
}

#[derive(Clone, Copy)]
struct CounterObservation<'a> {
    role: ObservationRole,
    owner: &'a str,
    thread_ids: &'a BTreeMap<String, AnalyticalId>,
    context: Option<&'a TurnContext>,
    delta: Option<crate::ledger::tokens::TokenMeasures>,
}

fn counter_observation(
    record: &ParsedRecord,
    last: &Value,
    total: &Value,
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
        observation.keys.push(COUNTER_KEY.key(vec![
            KeyComponent::text(thread_id.to_string()),
            KeyComponent::text(counter_signature(total)),
        ])?);
    }
    let mut usage = codex_usage(last)?;
    if let Some(delta) = counter.delta {
        usage.measures = delta;
    }
    observation.usage = Some(usage);
    observation.timestamp = record_timestamp(&record.value);
    if let Some(context) = counter.context {
        apply_context(&mut observation, context);
    }
    set_model_usage(&mut observation, "token_count.last_token_usage");
    Ok(observation)
}

fn append_limits(
    record: &ParsedRecord,
    owner: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    observations: &mut Vec<ProviderLimitObservation>,
) {
    let Some(rate_limits) = record.value.pointer("/payload/rate_limits") else {
        return;
    };
    if rate_limits.is_null() {
        return;
    }
    let limit_name = text(rate_limits, &["limit_id"])
        .or_else(|| text(rate_limits, &["limit_name"]))
        .map(str::to_owned);
    for window in ["primary", "secondary"] {
        let Some(_window_value) = rate_limits.get(window).filter(|value| !value.is_null()) else {
            continue;
        };
        let native = rate_limits
            .as_object()
            .map(|object| {
                object.iter().map(|(field, value)| (field.clone(), value.clone())).collect()
            })
            .unwrap_or_default();
        observations.push(ProviderLimitObservation {
            limit_name: limit_name.clone(),
            window: Some(window.to_owned()),
            observed_at: record_timestamp(&record.value).map_or(Basis::Unknown, Basis::Observed),
            owner_thread: thread_ids.get(owner).cloned(),
            owner_request: None,
            native,
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

fn counter_signature(total: &Value) -> String {
    [
        "input_tokens",
        "cached_input_tokens",
        "cache_write_input_tokens",
        "output_tokens",
        "reasoning_output_tokens",
        "total_tokens",
    ]
    .map(|field| {
        unsigned(total, &[field]).map_or_else(|| "?".to_owned(), |value| value.to_string())
    })
    .join(":")
}

fn direct_observation(
    record: &ParsedRecord,
    file_thread: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    turns: &BTreeMap<String, TurnContext>,
) -> Result<RequestObservation, AdapterError> {
    usage_observation(
        record,
        &record.value["payload"],
        ObservationRole::Original,
        file_thread,
        thread_ids,
        turns,
    )
}

fn usage_observation(
    record: &ParsedRecord,
    payload: &Value,
    default_role: ObservationRole,
    file_thread: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    turns: &BTreeMap<String, TurnContext>,
) -> Result<RequestObservation, AdapterError> {
    let owner = text(payload, &["thread_id"]).unwrap_or(file_thread);
    let mut observation = RequestObservation::new(record.evidence.clone(), DIALECT);
    observation.role = if default_role == ObservationRole::Copy || owner != file_thread {
        ObservationRole::Copy
    } else {
        ObservationRole::Original
    };
    observation.owner =
        thread_ids.get(owner).cloned().map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    if let Some(response_id) = text(payload, &["response_id"]) {
        observation.keys.push(
            RESPONSE_KEY.key(vec![
                KeyComponent::text(PROVIDER_NAMESPACE),
                KeyComponent::text(response_id),
            ])?,
        );
        observation.native_response_id = Some(response_id.to_owned());
    }
    if let Some(usage) = payload.get("usage") {
        observation.usage = Some(codex_usage(usage)?);
    }
    observation.timestamp = record_timestamp(&record.value);
    if let Some(context) = text(payload, &["root_turn_id"]).and_then(|turn| turns.get(turn)) {
        apply_context(&mut observation, context);
    }
    set_model_usage(&mut observation, "token_usage_record.usage");
    Ok(observation)
}

fn apply_context(observation: &mut RequestObservation, context: &TurnContext) {
    observation.model = context
        .model
        .as_ref()
        .map(|name| ModelName { name: name.clone(), basis: ModelBasis::Requested });
    observation.effort.clone_from(&context.effort);
}

fn set_model_usage(observation: &mut RequestObservation, source: &'static str) {
    if let Some(usage) = observation.usage.clone() {
        observation.model_usage.push(ModelUsage {
            model: observation.model.clone(),
            usage,
            source,
        });
    }
}

fn codex_usage(value: &Value) -> Result<TokenUsage, AdapterError> {
    let input = unsigned(value, &["input_tokens"]);
    let cache_read = unsigned(value, &["cached_input_tokens"]);
    let cache_write = unsigned(value, &["cache_write_input_tokens"]);
    let mut measures = normalize_input(
        InputSemantics::IncludesCacheRead,
        NativeInput { input, cache_read, cache_write },
    )?;
    measures.output = unsigned(value, &["output_tokens"]);
    measures.reasoning = unsigned(value, &["reasoning_output_tokens"]);
    let mut native = BTreeMap::new();
    for field in [
        "input_tokens",
        "cached_input_tokens",
        "cache_write_input_tokens",
        "output_tokens",
        "reasoning_output_tokens",
        "total_tokens",
    ] {
        if let Some(count) = unsigned(value, &[field]) {
            native.insert(field.to_owned(), count);
        }
    }
    Ok(TokenUsage { measures, native })
}

fn record_timestamp(value: &Value) -> Option<jiff::Timestamp> {
    text(value, &["timestamp"]).and_then(|timestamp| parse_timestamp(timestamp).ok())
}

fn thread_identity(native: &str) -> Result<StoredIdentity, AdapterError> {
    Ok(StoredIdentity::derive(IdentityKey::new(
        IdPrefix::Thread,
        "agent-thread",
        vec![KeyComponent::text(AGENT_NAMESPACE), KeyComponent::text(native)],
    ))?)
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
