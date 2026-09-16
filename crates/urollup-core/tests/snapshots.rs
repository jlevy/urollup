//! Stable snapshots of reconciled ledgers over representative frozen fixtures.

use std::path::PathBuf;

use serde_json::{Value, json};
use urollup_core::adapters::Ingested;
use urollup_core::adapters::claude_project;
use urollup_core::adapters::codex_rollout;
use urollup_core::ledger::entities::{
    AccountAttribution, Counting, ModelUsage, Ownership, Request, UsageRevision,
};
use urollup_core::ledger::identity::AnalyticalId;
use urollup_core::ledger::tokens::TokenUsage;
use urollup_core::sources::evidence::EvidenceRef;
use urollup_core::sources::manifest::{CoverageFailure, ManifestEntry, SourceChange};

fn fixture(dialect: &str, case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(dialect).join(case)
}

fn evidence(reference: &EvidenceRef) -> Value {
    json!({
        "length": reference.length,
        "offset": reference.offset,
        "source": reference.source.as_str(),
    })
}

fn usage(usage: &TokenUsage) -> Value {
    let measures: serde_json::Map<String, Value> = usage
        .measures
        .categories()
        .into_iter()
        .map(|(category, value)| (category.to_owned(), json!(value)))
        .collect();
    json!({
        "measures": measures,
        "native": usage.native,
    })
}

fn model_usage(component: &ModelUsage) -> Value {
    json!({
        "model": component.model.as_ref().map(|model| json!({
            "basis": format!("{:?}", model.basis),
            "name": model.name,
        })),
        "source": component.source,
        "usage": usage(&component.usage),
    })
}

fn revision(revision: &UsageRevision) -> Value {
    json!({
        "evidence": evidence(&revision.evidence),
        "model_usage": revision.model_usage.iter().map(model_usage).collect::<Vec<_>>(),
        "usage": usage(&revision.usage),
    })
}

fn ownership(ownership: &Ownership) -> Value {
    match ownership {
        Ownership::Owned { thread } => json!({ "owned": thread.as_str() }),
        Ownership::Ambiguous { candidates } => json!({
            "ambiguous": candidates.iter().map(AnalyticalId::as_str).collect::<Vec<_>>(),
        }),
        Ownership::Unknown => json!("unknown"),
    }
}

fn account(account: &AccountAttribution) -> Value {
    match account {
        AccountAttribution::Attributed(value) => json!({ "attributed": value }),
        AccountAttribution::Conflicting(values) => {
            json!({ "conflicting": values.iter().collect::<Vec<_>>() })
        }
        AccountAttribution::Unknown => json!("unknown"),
    }
}

fn counting(counting: &Counting) -> Value {
    match counting {
        Counting::Counted => json!("counted"),
        Counting::Unresolved { counted } => json!({ "unresolved": counted.as_str() }),
        Counting::CopyOnly => json!("copy-only"),
    }
}

fn request(request: &Request) -> Value {
    json!({
        "account": account(&request.account),
        "aliases": request.aliases.iter().map(|alias| alias.id.as_str()).collect::<Vec<_>>(),
        "basis": request.basis.token(),
        "copies": request.copies.iter().map(evidence).collect::<Vec<_>>(),
        "counting": counting(&request.counting),
        "effort": request.effort,
        "evidence": request.evidence.iter().map(evidence).collect::<Vec<_>>(),
        "first_seen": request.first_seen.map(|timestamp| timestamp.to_string()),
        "id": request.id().as_str(),
        "identity_key_kind": request.identity.key.kind,
        "last_seen": request.last_seen.map(|timestamp| timestamp.to_string()),
        "model": request.model.as_ref().map(|model| json!({
            "basis": format!("{:?}", model.basis),
            "name": model.name,
        })),
        "native_request_id": request.native_request_id,
        "native_response_id": request.native_response_id,
        "ownership": ownership(&request.ownership),
        "revisions": request.revisions.iter().map(revision).collect::<Vec<_>>(),
        "selected_usage": request.usage.as_ref().map(|selected| json!({
            "revision": revision(&selected.revision),
            "rule": selected.rule,
            "status": format!("{:?}", selected.status),
        })),
    })
}

fn coverage_failure(failure: &CoverageFailure) -> Value {
    match failure {
        CoverageFailure::Oversized { offset, length } => {
            json!({ "oversized": { "length": length, "offset": offset } })
        }
        CoverageFailure::CorruptCompressedData { decoded_offset, message } => json!({
            "corrupt_compressed_data": { "decoded_offset": decoded_offset, "message": message },
        }),
        CoverageFailure::IncompleteCompressedFrame { decoded_offset } => {
            json!({ "incomplete_compressed_frame": { "decoded_offset": decoded_offset } })
        }
        CoverageFailure::TwinFingerprintMismatch { path, locator } => json!({
            "twin_fingerprint_mismatch": {
                "file": path.file_name().map(|name| name.to_string_lossy()),
                "locator": locator,
            },
        }),
        CoverageFailure::ReadError { decoded_offset, kind } => json!({
            "read_error": { "decoded_offset": decoded_offset, "kind": format!("{kind:?}") },
        }),
    }
}

fn source_change(change: &SourceChange) -> Value {
    match change {
        SourceChange::BrieflyAbsent { attempts } => {
            json!({ "briefly_absent": { "attempts": attempts } })
        }
        SourceChange::Vanished => json!("vanished"),
        SourceChange::Replaced => json!("replaced"),
        SourceChange::Truncated { snapshot_len, observed_len } => json!({
            "truncated": { "observed_len": observed_len, "snapshot_len": snapshot_len },
        }),
        SourceChange::ModifiedInPlace => json!("modified_in_place"),
        SourceChange::FirstRecordChanged => json!("first_record_changed"),
        SourceChange::GrewBeyondCutoff { observed_len } => {
            json!({ "grew_beyond_cutoff": { "observed_len": observed_len } })
        }
    }
}

fn manifest_entry(entry: &ManifestEntry) -> Value {
    json!({
        "changes": entry.changes.iter().map(source_change).collect::<Vec<_>>(),
        "complete": entry.is_complete(),
        "counters": {
            "blank_lines": entry.counters.blank_lines,
            "decoded": entry.counters.decoded,
            "malformed": entry.counters.malformed,
            "oversized": entry.counters.oversized,
            "records": entry.counters.records,
            "skipped": entry.counters.skipped,
            "unparsable": entry.counters.unparsable,
        },
        "cutoff": {
            "complete_through": entry.cutoff.complete_through,
            "pending_tail": entry.cutoff.pending_tail.map(|tail| json!({
                "length": tail.length,
                "offset": tail.offset,
            })),
        },
        "dialect": entry.dialect,
        "environment": entry.environment,
        "failures": entry.failures.iter().map(coverage_failure).collect::<Vec<_>>(),
        "file_len": entry.file_len,
        "fingerprint": entry.fingerprint.map(|fingerprint| fingerprint.to_string()),
        "first_malformed": entry.first_malformed.as_ref().map(evidence),
        "locator": entry.locator,
        "representation": format!("{:?}", entry.representation),
        "source": entry.source.as_ref().map(|source| source.id.as_str()),
        "twin": entry.twin.is_some(),
    })
}

fn snapshot(ingested: &Ingested) -> Value {
    let coverage = &ingested.ledger.coverage;
    json!({
        "ledger": {
            "candidate_sets": ingested.ledger.candidate_sets.iter().map(|set| {
                set.iter().map(AnalyticalId::as_str).collect::<Vec<_>>()
            }).collect::<Vec<_>>(),
            "coverage": {
                "candidate_sets": coverage.candidate_sets,
                "conflicting_keys": coverage.conflicting_keys,
                "conflicting_rereads": coverage.conflicting_rereads,
                "copies": coverage.copies,
                "copy_only_requests": coverage.copy_only_requests,
                "observations": coverage.observations,
                "requests": coverage.requests,
                "requests_without_usage": coverage.requests_without_usage,
                "rereads": coverage.rereads,
                "unresolved_requests": coverage.unresolved_requests,
            },
            "diagnostics": ingested.ledger.diagnostics.iter().map(|diagnostic| json!({
                "code": diagnostic.code.token(),
                "detail": diagnostic.detail,
                "evidence": diagnostic.evidence.iter().map(evidence).collect::<Vec<_>>(),
                "occurrences": diagnostic.occurrences,
                "subject": diagnostic.subject.as_ref().map(AnalyticalId::as_str),
            })).collect::<Vec<_>>(),
            "gaps": ingested.ledger.gaps.iter().map(|gap| json!({
                "evidence": gap.evidence.iter().map(evidence).collect::<Vec<_>>(),
                "reason": format!("{:?}", gap.reason),
                "thread": gap.thread.as_ref().map(AnalyticalId::as_str),
            })).collect::<Vec<_>>(),
            "requests": ingested.ledger.requests.values().map(request).collect::<Vec<_>>(),
        },
        "manifest": {
            "entries": ingested.manifest.entries.iter().map(manifest_entry).collect::<Vec<_>>(),
            "skipped_links": ingested.manifest.skipped_links.iter().map(|link| json!({
                "file": link.path.file_name().map(|name| name.to_string_lossy()),
                "reason": format!("{:?}", link.reason),
            })).collect::<Vec<_>>(),
        },
    })
}

#[test]
fn claude_double_counting_snapshot() {
    let ingested =
        claude_project::ingest_root(&fixture("claude-project", "brief-double-counting")).unwrap();

    insta::assert_yaml_snapshot!(snapshot(&ingested));
}

#[test]
fn codex_repeated_cumulative_snapshot() {
    let ingested =
        codex_rollout::ingest_root(&fixture("codex-rollout", "brief-repeated-snapshot")).unwrap();

    insta::assert_yaml_snapshot!(snapshot(&ingested));
}

#[test]
fn codex_compressed_twin_snapshot() {
    let ingested = codex_rollout::ingest_root(&fixture("codex-rollout", "zst-twin")).unwrap();

    insta::assert_yaml_snapshot!(snapshot(&ingested));
}
