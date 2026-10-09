//! Explicit fork boundaries must work without an embedded parent session header.

use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroUsize;

use serde_json::json;
use urollup_core::accounting::totals::{
    Completeness, PartialReason, SelectionTotals, ledger_totals, selection_totals,
};
use urollup_core::adapters::{Ingested, codex_rollout};
use urollup_core::ledger::coverage::UnobservedReason;
use urollup_core::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use urollup_core::ledger::entities::{Counting, Ownership};
use urollup_core::ledger::identity::AnalyticalId;
use urollup_core::ledger::tokens::TokenMeasures;
use urollup_core::selection::{Agent, agent_thread_identity};
use urollup_core::sources::roots::discover;

const PARENT: &str = "11111111-1111-4111-8111-111111111111";
const CHILD: &str = "22222222-2222-4222-8222-222222222222";
const OTHER: &str = "33333333-3333-4333-8333-333333333333";

fn counter(ordinal: u64, input: u64, output: u64, last_input: u64, last_output: u64) -> String {
    json!({
        "ordinal": ordinal,
        "timestamp": "2026-01-01T00:00:01Z",
        "type": "event_msg",
        "payload": {
            "type": "token_count",
            "info": {
                "total_token_usage": {
                    "input_tokens": input, "cached_input_tokens": 0,
                    "output_tokens": output, "reasoning_output_tokens": 0,
                    "total_tokens": input + output
                },
                "last_token_usage": {
                    "input_tokens": last_input, "cached_input_tokens": 0,
                    "output_tokens": last_output, "reasoning_output_tokens": 0,
                    "total_tokens": last_input + last_output
                }
            }
        }
    })
    .to_string()
}

fn fork_root(parent_records: Option<&[String]>, child_records: &[String]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temporary log root");
    let sessions = root.path().join("sessions");
    fs::create_dir(&sessions).expect("sessions directory");
    let parent_meta = json!({"ordinal": 0, "type": "session_meta", "payload": {"id": PARENT}});
    let child_meta = json!({"ordinal": 0, "type": "session_meta", "payload": {
        "id": CHILD, "parent_thread_id": PARENT, "forked_from_id": PARENT,
        "history_mode": "paginated", "subagent_history_start_ordinal": 3
    }});
    if let Some(records) = parent_records {
        fs::write(
            sessions.join(format!("rollout-2026-01-01T00-00-00-{PARENT}.jsonl")),
            format!("{parent_meta}\n{}\n", records.join("\n")),
        )
        .expect("parent rollout");
    }
    fs::write(
        sessions.join(format!("rollout-2026-01-01T00-00-02-{CHILD}.jsonl")),
        format!("{child_meta}\n{}\n", child_records.join("\n")),
    )
    .expect("child rollout");
    root
}

fn counted_totals(ingested: &urollup_core::adapters::Ingested) -> Vec<u64> {
    ingested
        .ledger
        .requests
        .values()
        .filter(|request| request.counting == Counting::Counted)
        .map(|request| {
            TokenMeasures::from(request.usage.as_ref().expect("counted usage").revision.usage)
                .total()
                .expect("valid sum")
                .expect("known tokens")
        })
        .collect()
}

#[test]
fn paginated_counter_prefix_is_not_counted_as_child_usage() {
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 108, 12, 18, 2)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let totals = counted_totals(&ingested);
    assert_eq!(totals.len(), 2, "one parent request and one child request");
    assert_eq!(totals.iter().sum::<u64>(), 120, "inherited usage counts once");
    assert!(totals.contains(&20), "the child's own usage excludes its baseline");
}

/// A `token_usage_record` as Codex writes it, naming the thread that made the request.
fn direct(ordinal: u64, thread: &str, response: &str, input: u64, output: u64) -> String {
    json!({"ordinal": ordinal, "type": "token_usage_record", "payload": {
        "thread_id": thread, "response_id": response, "usage": {
            "input_tokens": input, "cached_input_tokens": 0,
            "output_tokens": output, "reasoning_output_tokens": 0,
            "total_tokens": input + output
        }
    }})
    .to_string()
}

fn compacted_parent() -> String {
    let record: serde_json::Value =
        serde_json::from_str(&direct(2, PARENT, "parent-response", 90, 10))
            .expect("synthetic direct record");
    json!({"ordinal": 2, "type": "compacted", "payload": {
        "latest_token_usage_record": record["payload"]
    }})
    .to_string()
}

#[test]
fn paginated_direct_prefix_without_a_parent_log_stays_copy_only() {
    let root = fork_root(
        None,
        &[direct(1, PARENT, "parent-response", 90, 10), direct(3, CHILD, "child-response", 18, 2)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    assert_eq!(counted_totals(&ingested), [20]);
    assert_eq!(
        ingested
            .ledger
            .requests
            .values()
            .filter(|request| request.counting == Counting::CopyOnly)
            .count(),
        1
    );
}

fn unpositioned(record: &str) -> String {
    let mut record: serde_json::Value = serde_json::from_str(record).expect("synthetic record");
    record.as_object_mut().expect("record object").remove("ordinal");
    record.to_string()
}

/// A `token_count` that reports only rate limits, written without an ordinal.
fn limits_only() -> String {
    json!({"timestamp": "2026-01-01T00:00:01Z", "type": "event_msg", "payload": {
        "type": "token_count", "info": null, "rate_limits": {
            "limit_id": "codex", "limit_name": null,
            "primary": {"used_percent": 41, "window_minutes": 300, "resets_at": 1_788_382_800},
            "secondary": null
        }
    }})
    .to_string()
}

/// Writes the child rollout with its own `session_meta` payload fields.
fn write_child(root: &tempfile::TempDir, meta: serde_json::Value, records: &[String]) {
    let serde_json::Value::Object(fields) = meta else { panic!("meta is an object") };
    let mut payload = json!({"id": CHILD});
    payload.as_object_mut().expect("payload").extend(fields);
    let metadata = json!({"ordinal": 0, "type": "session_meta", "payload": payload});
    fs::write(
        root.path().join("sessions").join(format!("rollout-2026-01-01T00-00-02-{CHILD}.jsonl")),
        format!("{metadata}\n{}\n", records.join("\n")),
    )
    .expect("child rollout");
}

fn thread(native: &str) -> AnalyticalId {
    agent_thread_identity(Agent::Codex, native).expect("thread identity").id
}

fn select(ingested: &Ingested, threads: &[&str]) -> SelectionTotals {
    let threads = threads.iter().map(|native| thread(native)).collect();
    selection_totals(&ingested.ledger, &threads).expect("selection totals")
}

fn boundary_diagnostics(ingested: &Ingested) -> Vec<&Diagnostic> {
    ingested
        .ledger
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagnosticCode::CodexHistoryBoundaryUnverified)
        .collect()
}

/// The thread's unverifiable usage is excluded: a diagnostic names the thread, and a
/// coverage gap makes its selection and the whole history partial.
fn assert_excluded_as_gap(ingested: &Ingested, native: &str) {
    let thread = thread(native);
    let diagnostics = boundary_diagnostics(ingested);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].subject.as_ref(), Some(&thread));
    assert!(!diagnostics[0].evidence.is_empty(), "the diagnostic cites the records");
    assert_eq!(
        ingested
            .ledger
            .gaps
            .iter()
            .map(|gap| (&gap.reason, gap.thread.as_ref()))
            .collect::<Vec<_>>(),
        [(&UnobservedReason::UnverifiedHistoryBoundary, Some(&thread))]
    );
    let partial = Completeness::Partial(BTreeSet::from([PartialReason::UnobservedGap]));
    assert_eq!(select(ingested, &[native]).completeness, partial);
    assert_eq!(ledger_totals(&ingested.ledger).expect("totals").completeness, partial);
}

#[test]
fn unpositioned_usage_under_an_explicit_boundary_is_excluded_as_a_coverage_gap() {
    for record in [counter(3, 18, 2, 18, 2), direct(3, CHILD, "child-response", 18, 2)] {
        let root = fork_root(None, &[unpositioned(&record)]);
        let ingested =
            codex_rollout::ingest_root(root.path()).expect("one rollout's anomaly is reportable");
        assert_eq!(counted_totals(&ingested), [] as [u64; 0], "unplaced usage never counts");
        assert_excluded_as_gap(&ingested, CHILD);
    }
}

#[test]
fn an_unpositioned_counter_after_child_usage_rebases_rather_than_merging_into_the_next_step() {
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[
            inherited.clone(),
            counter(3, 108, 12, 18, 2),
            unpositioned(&counter(4, 120, 15, 12, 3)),
            counter(5, 130, 17, 10, 2),
        ],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let mut totals = counted_totals(&ingested);
    totals.sort_unstable();
    assert_eq!(totals, [12, 20, 100], "the unplaced step's 15 tokens are not in the next delta");
    assert_excluded_as_gap(&ingested, CHILD);
}

#[test]
fn a_limits_only_token_count_without_an_ordinal_is_not_a_boundary_anomaly() {
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), limits_only(), counter(3, 108, 12, 18, 2)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 120);
    assert!(boundary_diagnostics(&ingested).is_empty());
    assert!(ingested.ledger.gaps.is_empty());
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
    assert!(!ingested.limit_observations.is_empty(), "the limits are still observed");
}

#[test]
fn a_child_counter_without_its_inherited_baseline_is_excluded_as_a_coverage_gap() {
    for parent_known in [true, false] {
        let root = fork_root(None, &[]);
        let meta = if parent_known {
            json!({"parent_thread_id": PARENT, "subagent_history_start_ordinal": 3})
        } else {
            json!({"subagent_history_start_ordinal": 3})
        };
        write_child(&root, meta, &[counter(3, 108, 12, 18, 2), counter(4, 113, 13, 5, 1)]);
        let ingested =
            codex_rollout::ingest_root(root.path()).expect("one rollout's anomaly is reportable");
        assert_eq!(
            counted_totals(&ingested),
            [6],
            "the unverifiable first step is excluded and later deltas still count"
        );
        assert_excluded_as_gap(&ingested, CHILD);
    }
}

#[test]
fn one_unverifiable_rollout_does_not_hide_other_sessions() {
    let root = fork_root(None, &[]);
    write_child(
        &root,
        json!({"parent_thread_id": PARENT, "subagent_history_start_ordinal": 3}),
        &[counter(3, 108, 12, 18, 2)],
    );
    let other = json!({"type": "session_meta", "payload": {"id": OTHER}});
    let mut other_count: serde_json::Value =
        serde_json::from_str(&counter(0, 40, 4, 40, 4)).expect("synthetic counter");
    other_count.as_object_mut().expect("record object").remove("ordinal");
    fs::write(
        root.path().join("sessions").join(format!("rollout-2026-01-01T00-00-01-{OTHER}.jsonl")),
        format!("{other}\n{other_count}\n"),
    )
    .expect("healthy rollout");
    let ingested =
        codex_rollout::ingest_root(root.path()).expect("the healthy session still reports");
    assert_eq!(counted_totals(&ingested), [44]);
    let healthy = select(&ingested, &[OTHER]);
    assert_eq!((healthy.counted.requests, healthy.completeness), (1, Completeness::Complete));
    assert_excluded_as_gap(&ingested, CHILD);
}

#[test]
fn an_unseeded_first_step_after_an_inherited_baseline_counts_its_own_usage() {
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 300, 50, 300, 50)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let child = select(&ingested, &[CHILD]);
    assert_eq!(child.counted.tokens.total().expect("valid sum"), Some(350));
    assert_eq!(child.completeness, Completeness::Complete);
    assert!(
        ingested
            .ledger
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::CodexCounterEpochReset
                && diagnostic.subject == Some(thread(CHILD))),
        "an unseeded child opens a new counter epoch"
    );
}

#[test]
fn a_first_step_inconsistent_with_its_inherited_baseline_is_excluded_as_a_coverage_gap() {
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 150, 20, 30, 5), counter(4, 160, 22, 10, 2)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let mut totals = counted_totals(&ingested);
    totals.sort_unstable();
    assert_eq!(totals, [12, 100], "neither 70 nor 35 tokens is proven the child's own");
    assert_excluded_as_gap(&ingested, CHILD);
}

#[test]
fn a_prefix_without_parent_identity_is_excluded_without_losing_child_usage() {
    for records in [
        vec![counter(1, 90, 10, 90, 10), counter(3, 108, 12, 18, 2)],
        vec![
            direct(1, PARENT, "parent-response", 90, 10),
            direct(3, CHILD, "child-response", 18, 2),
        ],
    ] {
        let root = fork_root(None, &[]);
        let child_path =
            root.path().join("sessions").join(format!("rollout-2026-01-01T00-00-02-{CHILD}.jsonl"));
        let metadata = json!({"ordinal": 0, "type": "session_meta", "payload": {
            "id": CHILD, "subagent_history_start_ordinal": 3
        }});
        fs::write(child_path, format!("{metadata}\n{}\n", records.join("\n")))
            .expect("unattributed prefix");
        let ingested =
            codex_rollout::ingest_root(root.path()).expect("unowned copies are reportable");
        assert_eq!(counted_totals(&ingested), [20]);
        let copies: Vec<_> = ingested
            .ledger
            .requests
            .values()
            .filter(|request| request.counting == Counting::CopyOnly)
            .collect();
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].ownership, Ownership::Unknown);
        // Copy-only requests are excluded from totals but do not change completeness.
        let totals = urollup_core::accounting::totals::ledger_totals(&ingested.ledger).unwrap();
        assert_eq!(totals.completeness, urollup_core::accounting::totals::Completeness::Complete);
        assert_eq!(totals.copy_only.requests, 1);
    }
}

#[test]
fn malformed_explicit_boundaries_do_not_fall_back_to_child_ownership() {
    for boundary in [json!("3"), json!(-1), json!(1.5), json!({}), json!([]), json!(true)] {
        let root = fork_root(None, &[]);
        write_child(
            &root,
            json!({"parent_thread_id": PARENT, "subagent_history_start_ordinal": boundary}),
            &[counter(1, 90, 10, 90, 10), counter(3, 108, 12, 18, 2)],
        );
        let ingested =
            codex_rollout::ingest_root(root.path()).expect("one rollout's anomaly is reportable");
        assert_eq!(counted_totals(&ingested), [] as [u64; 0], "boundary {boundary}");
        assert_excluded_as_gap(&ingested, CHILD);
    }
}

#[test]
fn a_child_starting_from_zero_without_a_logged_prefix_reports_no_excluded_copies() {
    let root = fork_root(None, &[counter(3, 18, 2, 18, 2)]);
    let ingested = codex_rollout::ingest_root(root.path()).expect("known zero baseline");
    assert_eq!(counted_totals(&ingested), [20]);
    assert_eq!(ingested.ledger.coverage.copies, 0);
}

#[test]
fn fork_totals_and_ownership_are_invariant_to_workers_source_order_and_parent_presence() {
    let child_id = agent_thread_identity(Agent::Codex, CHILD).expect("child identity").id;
    for direct_usage in [false, true] {
        for parent_present in [false, true] {
            for reset in [false, true] {
                let inherited = counter(1, 90, 10, 90, 10);
                let next =
                    if reset { counter(3, 18, 2, 18, 2) } else { counter(3, 108, 12, 18, 2) };
                let (parent, child) = if direct_usage {
                    (
                        vec![direct(1, PARENT, "parent-response", 90, 10), inherited],
                        vec![
                            direct(1, PARENT, "parent-response", 90, 10),
                            counter(2, 90, 10, 90, 10),
                            compacted_parent(),
                            direct(3, CHILD, "child-response", 18, 2),
                            next,
                        ],
                    )
                } else {
                    (vec![inherited.clone()], vec![inherited, next])
                };
                let root = fork_root(parent_present.then_some(parent.as_slice()), &child);
                let roots = codex_rollout::rollout_roots(&[root.path().to_owned()]);
                let mut baseline = None;
                for workers in [1, 8] {
                    for reversed in [false, true] {
                        let mut discovery = discover(&roots);
                        if reversed {
                            discovery.sources.reverse();
                        }
                        let ingested = codex_rollout::ingest_discovery_with_workers(
                            discovery,
                            true,
                            NonZeroUsize::new(workers).expect("positive workers"),
                        )
                        .expect("ingest fork matrix");
                        let totals = counted_totals(&ingested);
                        assert_eq!(
                            totals.iter().sum::<u64>(),
                            if parent_present { 120 } else { 20 }
                        );
                        assert_eq!(totals.len(), if parent_present { 2 } else { 1 });
                        let coverage =
                            urollup_core::accounting::totals::ledger_totals(&ingested.ledger)
                                .expect("fork totals")
                                .completeness;
                        assert_eq!(
                            coverage,
                            urollup_core::accounting::totals::Completeness::Complete,
                            "excluded copies never make coverage incomplete"
                        );
                        assert!(
                            ingested
                                .ledger
                                .requests
                                .values()
                                .filter(|request| request.counting == Counting::Counted)
                                .all(|request| matches!(
                                    request.ownership,
                                    Ownership::Owned { .. }
                                )),
                            "copies must not introduce conflicting owners"
                        );
                        let child_requests: Vec<_> = ingested
                            .ledger
                            .requests
                            .values()
                            .filter(|request| {
                                request.counting == Counting::Counted
                                    && request.ownership
                                        == Ownership::Owned { thread: child_id.clone() }
                            })
                            .collect();
                        assert_eq!(child_requests.len(), 1);
                        let usage =
                            &child_requests[0].usage.as_ref().expect("child usage").revision.usage;
                        assert_eq!(
                            TokenMeasures::from(*usage).total().expect("valid sum"),
                            Some(20)
                        );
                        if let Some(expected) = &baseline {
                            assert_eq!(
                                &ingested.ledger, expected,
                                "workers={workers}, reversed={reversed}"
                            );
                        } else {
                            baseline = Some(ingested.ledger);
                        }
                    }
                }
            }
        }
    }
}
