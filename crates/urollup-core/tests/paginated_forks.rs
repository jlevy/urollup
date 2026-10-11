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
use urollup_core::ledger::entities::{Counting, Ownership, RelationshipKind};
use urollup_core::ledger::identity::AnalyticalId;
use urollup_core::ledger::scope::IdentityBasis;
use urollup_core::ledger::tokens::TokenMeasures;
use urollup_core::selection::{Agent, Scope, SelectionQuery, SessionIndex, agent_thread_identity};
use urollup_core::sources::roots::discover;

const PARENT: &str = "11111111-1111-4111-8111-111111111111";
const CHILD: &str = "22222222-2222-4222-8222-222222222222";
const OTHER: &str = "33333333-3333-4333-8333-333333333333";
const GRANDCHILD: &str = "44444444-4444-4444-8444-444444444444";

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

/// Writes the rollout of thread `native`, named for `second`, whose own `session_meta` at
/// ordinal 0 adds the `meta` payload fields.
fn write_rollout(
    root: &tempfile::TempDir,
    native: &str,
    second: u8,
    meta: serde_json::Value,
    records: &[String],
) {
    let serde_json::Value::Object(fields) = meta else { panic!("meta is an object") };
    let mut payload = json!({"id": native});
    payload.as_object_mut().expect("payload").extend(fields);
    let metadata = json!({"ordinal": 0, "type": "session_meta", "payload": payload});
    fs::write(
        root.path()
            .join("sessions")
            .join(format!("rollout-2026-01-01T00-00-{second:02}-{native}.jsonl")),
        format!("{metadata}\n{}\n", records.join("\n")),
    )
    .expect("synthetic rollout");
}

/// Writes the child rollout with its own `session_meta` payload fields.
fn write_child(root: &tempfile::TempDir, meta: serde_json::Value, records: &[String]) {
    write_rollout(root, CHILD, 2, meta, records);
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
    let root = fork_root(None, &[unpositioned(&counter(3, 18, 2, 18, 2))]);
    let ingested =
        codex_rollout::ingest_root(root.path()).expect("one rollout's anomaly is reportable");
    assert_eq!(counted_totals(&ingested), [] as [u64; 0], "unplaced usage never counts");
    assert_excluded_as_gap(&ingested, CHILD);
}

#[test]
fn a_record_naming_its_own_thread_counts_without_a_placeable_position() {
    // Codex writes the recording thread into every token_usage_record, and a copy keeps its
    // original's thread, so a record that names the rollout's own thread is that thread's
    // request whether or not the declared boundary can place it.
    let unpositioned = fork_root(None, &[unpositioned(&direct(3, CHILD, "child-response", 18, 2))]);
    let invalid = fork_root(None, &[]);
    write_child(
        &invalid,
        json!({"parent_thread_id": PARENT, "subagent_history_start_ordinal": "3"}),
        &[direct(1, CHILD, "child-response", 18, 2)],
    );
    for root in [unpositioned, invalid] {
        let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
        assert_eq!(counted_totals(&ingested), [20]);
        assert!(boundary_diagnostics(&ingested).is_empty());
        assert!(ingested.ledger.gaps.is_empty());
        assert_eq!(
            ledger_totals(&ingested.ledger).expect("totals").completeness,
            Completeness::Complete
        );
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
            // Counter path: the child's counter continues the inherited total or restarts
            // from zero. Direct path: the copied prefix still holds the parent's usage
            // record, which keys the copied token_count (before 0.153), or holds only the
            // token_count, which stays unkeyed (0.153 and later).
            for variant in [false, true] {
                let inherited = counter(1, 90, 10, 90, 10);
                let next =
                    if variant { counter(3, 18, 2, 18, 2) } else { counter(3, 108, 12, 18, 2) };
                let (parent, child) = if direct_usage {
                    let mut child = vec![
                        direct(1, PARENT, "parent-response", 90, 10),
                        counter(2, 90, 10, 90, 10),
                        compacted_parent(),
                        direct(3, CHILD, "child-response", 18, 2),
                        counter(3, 108, 12, 18, 2),
                    ];
                    if variant {
                        child.remove(0);
                    }
                    (vec![direct(1, PARENT, "parent-response", 90, 10), inherited], child)
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

/// The `uro-kpbp` fork on the counter path and on the direct path: a parent with 100
/// tokens and a child with a copied prefix and 20 tokens of its own.
fn kpbp_fork(direct_usage: bool) -> tempfile::TempDir {
    let inherited = counter(1, 90, 10, 90, 10);
    if direct_usage {
        let parent = [direct(1, PARENT, "parent-response", 90, 10), counter(2, 90, 10, 90, 10)];
        let root = fork_root(Some(&parent), &[]);
        write_child(
            &root,
            json!({"parent_thread_id": PARENT, "forked_from_id": PARENT,
                   "history_mode": "paginated", "subagent_history_start_ordinal": 3}),
            &[
                direct(1, PARENT, "parent-response", 90, 10),
                counter(2, 90, 10, 90, 10),
                direct(3, CHILD, "child-response", 18, 2),
                counter(4, 108, 12, 18, 2),
            ],
        );
        root
    } else {
        fork_root(
            Some(std::slice::from_ref(&inherited)),
            &[inherited.clone(), counter(3, 108, 12, 18, 2)],
        )
    }
}

#[test]
fn self_combined_and_descendant_selections_count_the_inherited_prefix_once() {
    for direct_usage in [false, true] {
        let root = kpbp_fork(direct_usage);
        let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
        assert!(
            ingested
                .relationships
                .iter()
                .any(|relationship| relationship.kind == RelationshipKind::Spawn
                    && relationship.from == thread(PARENT)
                    && relationship.to == thread(CHILD)),
            "the child is a descendant of its parent"
        );
        let counted = |threads: &[&str]| {
            let totals = select(&ingested, threads);
            (totals.counted.requests, totals.counted.tokens.total().expect("valid sum"))
        };
        assert_eq!(counted(&[PARENT]), (1, Some(100)), "parent self, direct={direct_usage}");
        assert_eq!(counted(&[CHILD]), (1, Some(20)), "child self, direct={direct_usage}");
        assert_eq!(counted(&[PARENT, CHILD]), (2, Some(120)), "combined, direct={direct_usage}");

        let mut index = SessionIndex::default();
        index.add(Agent::Codex, &ingested).expect("index synthetic fork");
        for (scope, expected) in [(Scope::SelfOnly, 100), (Scope::Descendants, 120)] {
            let selected = index
                .select(&SelectionQuery {
                    sessions: vec![PARENT.into()],
                    scope: Some(scope),
                    ..SelectionQuery::default()
                })
                .expect("select the parent");
            let totals = selection_totals(&ingested.ledger, &selected).expect("selection totals");
            assert_eq!(
                (totals.counted.tokens.total().expect("valid sum"), totals.completeness),
                (Some(expected), Completeness::Complete),
                "{scope:?}, direct={direct_usage}"
            );
        }
    }
}

#[test]
fn a_counter_reset_after_the_child_advanced_opens_a_new_epoch_for_the_child() {
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 108, 12, 18, 2), counter(4, 5, 1, 5, 1)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let child = select(&ingested, &[CHILD]);
    assert_eq!(
        (child.counted.requests, child.counted.tokens.total().expect("valid sum")),
        (2, Some(26)),
        "20 before the reset and 6 after it"
    );
    assert_eq!(select(&ingested, &[PARENT]).counted.tokens.total().expect("valid sum"), Some(100));
    assert!(
        ingested
            .ledger
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::CodexCounterEpochReset
                && diagnostic.subject == Some(thread(CHILD)))
    );
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
}

#[test]
fn an_unkeyed_copied_token_count_with_its_parent_present_keeps_coverage_complete() {
    // From 0.153 forked prefixes keep the parent's token_count but drop its usage record,
    // so the copy has no response key to reconcile with. Paginated (ordinals and an
    // explicit boundary) and legacy (embedded parent header, then a settings event that
    // names the child) destinations both hold such a copy.
    let parent = [direct(1, PARENT, "parent-response", 90, 10), counter(2, 90, 10, 90, 10)];
    let paginated = fork_root(Some(&parent), &[]);
    write_child(
        &paginated,
        json!({"parent_thread_id": PARENT, "subagent_history_start_ordinal": 2}),
        &[
            counter(1, 90, 10, 90, 10),
            direct(2, CHILD, "child-response", 18, 2),
            counter(3, 108, 12, 18, 2),
        ],
    );
    let legacy = fork_root(Some(&parent), &[]);
    write_child(
        &legacy,
        json!({"parent_thread_id": PARENT}),
        &[
            json!({"type": "session_meta", "payload": {"id": PARENT}}).to_string(),
            unpositioned(&counter(0, 90, 10, 90, 10)),
            json!({"type": "event_msg", "payload": {
                "type": "thread_settings_applied", "thread_id": CHILD
            }})
            .to_string(),
            unpositioned(&direct(0, CHILD, "child-response", 18, 2)),
            unpositioned(&counter(0, 108, 12, 18, 2)),
        ],
    );
    // The paginated destination's declared boundary also counts its copied region.
    for (root, copies) in [(paginated, 2), (legacy, 1)] {
        let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
        let mut totals = counted_totals(&ingested);
        totals.sort_unstable();
        assert_eq!(totals, [20, 100]);
        let totals = ledger_totals(&ingested.ledger).expect("totals");
        assert_eq!(totals.copy_only.requests, 1, "the unkeyed copy is excluded");
        assert_eq!(totals.completeness, Completeness::Complete);
        assert_eq!(select(&ingested, &[PARENT]).completeness, Completeness::Complete);
        let copy = ingested
            .ledger
            .requests
            .values()
            .find(|request| request.counting == Counting::CopyOnly)
            .expect("the copy-only request");
        assert_eq!(copy.basis, IdentityBasis::Ambiguous, "the copy has no key");
        assert_eq!(ingested.ledger.coverage.copies, copies);
        assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
    }
}

#[test]
fn nested_paginated_subagents_with_every_original_present_report_complete_coverage() {
    let root = tempfile::tempdir().expect("temporary log root");
    fs::create_dir(root.path().join("sessions")).expect("sessions directory");
    write_rollout(&root, PARENT, 0, json!({}), &[counter(1, 90, 10, 90, 10)]);
    write_rollout(
        &root,
        CHILD,
        1,
        json!({"parent_thread_id": PARENT, "subagent_history_start_ordinal": 2}),
        &[counter(1, 90, 10, 90, 10), counter(2, 108, 12, 18, 2)],
    );
    write_rollout(
        &root,
        GRANDCHILD,
        2,
        json!({"parent_thread_id": CHILD, "subagent_history_start_ordinal": 3}),
        &[counter(1, 90, 10, 90, 10), counter(2, 108, 12, 18, 2), counter(3, 113, 13, 5, 1)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest nested forks");
    let mut totals = counted_totals(&ingested);
    totals.sort_unstable();
    assert_eq!(totals, [6, 20, 100]);
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
    for native in [PARENT, CHILD, GRANDCHILD] {
        assert_eq!(select(&ingested, &[native]).completeness, Completeness::Complete, "{native}");
    }
}

#[test]
fn an_unpositioned_token_count_repeating_the_running_total_adds_nothing_and_is_no_gap() {
    // Codex re-sends the current totals with every rate-limit refresh, so a token_count
    // without an ordinal may only repeat the inherited or the running total.
    let inherited = counter(1, 90, 10, 90, 10);
    for child in [
        vec![
            inherited.clone(),
            unpositioned(&counter(0, 90, 10, 90, 10)),
            counter(3, 108, 12, 18, 2),
            counter(5, 113, 13, 5, 1),
        ],
        vec![
            inherited.clone(),
            counter(3, 108, 12, 18, 2),
            unpositioned(&counter(0, 108, 12, 18, 2)),
            counter(5, 113, 13, 5, 1),
        ],
    ] {
        let root = fork_root(Some(std::slice::from_ref(&inherited)), &child);
        let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
        let mut totals = counted_totals(&ingested);
        totals.sort_unstable();
        assert_eq!(totals, [6, 20, 100]);
        assert!(boundary_diagnostics(&ingested).is_empty(), "a repeated total is no anomaly");
        assert!(ingested.ledger.gaps.is_empty());
        assert_eq!(
            ledger_totals(&ingested.ledger).expect("totals").completeness,
            Completeness::Complete
        );
    }
}

#[test]
fn an_explicit_boundary_of_zero_still_checks_the_first_counter_step() {
    let parent = [counter(1, 90, 10, 90, 10)];
    let meta = json!({"parent_thread_id": PARENT, "subagent_history_start_ordinal": 0});

    let seeded = fork_root(Some(&parent), &[]);
    write_child(&seeded, meta.clone(), &[counter(1, 108, 12, 18, 2)]);
    let ingested = codex_rollout::ingest_root(seeded.path()).expect("ingest synthetic fork");
    assert_eq!(counted_totals(&ingested), [100], "the inherited 100 is never the child's");
    assert_excluded_as_gap(&ingested, CHILD);

    let unseeded = fork_root(Some(&parent), &[]);
    write_child(&unseeded, meta, &[counter(1, 18, 2, 18, 2)]);
    let ingested = codex_rollout::ingest_root(unseeded.path()).expect("ingest synthetic fork");
    let mut totals = counted_totals(&ingested);
    totals.sort_unstable();
    assert_eq!(totals, [20, 100], "a child that starts from zero passes the check");
    assert!(boundary_diagnostics(&ingested).is_empty());
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
}

#[test]
fn a_repeated_inherited_total_does_not_use_up_the_first_step_check() {
    // A rate-limit refresh after the boundary repeats the inherited total and reports no
    // usage, so the next step that does is still checked against the baseline.
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 90, 10, 90, 10), counter(4, 150, 20, 30, 5)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    assert_eq!(counted_totals(&ingested), [100], "the inconsistent step stays excluded");
    assert_excluded_as_gap(&ingested, CHILD);
}

fn reset_diagnostics(ingested: &Ingested, native: &str) -> usize {
    let thread = thread(native);
    ingested
        .ledger
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.code == DiagnosticCode::CodexCounterEpochReset
                && diagnostic.subject.as_ref() == Some(&thread)
        })
        .count()
}

#[test]
fn a_lowered_total_after_the_childs_first_step_counts_only_its_own_last_usage() {
    // Once the first step proves the child's baseline, a lowered total is an ordinary
    // decrease within the child's own epoch: it counts its own last usage, or adds no
    // request when that has no input or output, and never charges the lowered total.
    let inherited = counter(1, 90, 10, 90, 10);
    for (lowered, expected) in
        [(counter(4, 60, 13, 4, 1), (3, Some(37))), (counter(4, 60, 13, 0, 0), (2, Some(32)))]
    {
        let root = fork_root(
            Some(std::slice::from_ref(&inherited)),
            &[inherited.clone(), counter(3, 108, 12, 18, 2), lowered, counter(5, 70, 15, 10, 2)],
        );
        let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
        let child = select(&ingested, &[CHILD]);
        assert_eq!(
            (child.counted.requests, child.counted.tokens.total().expect("valid sum")),
            expected,
            "20 at the first step, the lowered step's own usage, then 12"
        );
        assert_eq!(
            select(&ingested, &[PARENT]).counted.tokens.total().expect("valid sum"),
            Some(100)
        );
        assert_eq!(reset_diagnostics(&ingested, CHILD), 1);
        assert!(boundary_diagnostics(&ingested).is_empty());
        assert_eq!(
            ledger_totals(&ingested.ledger).expect("totals").completeness,
            Completeness::Complete
        );
    }
}

#[test]
fn a_first_step_below_the_inherited_total_counts_only_as_its_own_whole_usage() {
    // The first-step check, not the rule for a total lowered at compaction, decides a
    // first own step that falls below the inherited total: it has no delta, so its whole
    // total must be its own last usage.
    let inherited = counter(1, 90, 10, 90, 10);
    let restarted = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 18, 2, 18, 2), counter(4, 30, 5, 12, 3)],
    );
    let ingested = codex_rollout::ingest_root(restarted.path()).expect("ingest synthetic fork");
    let child = select(&ingested, &[CHILD]);
    assert_eq!(
        (child.counted.requests, child.counted.tokens.total().expect("valid sum")),
        (2, Some(35)),
        "a counter that restarted from zero counts 20 once, then 15"
    );
    assert_eq!(reset_diagnostics(&ingested, CHILD), 1, "one decrease, one diagnostic");
    assert!(boundary_diagnostics(&ingested).is_empty());
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );

    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 60, 8, 5, 1), counter(4, 70, 10, 10, 2)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let mut totals = counted_totals(&ingested);
    totals.sort_unstable();
    assert_eq!(totals, [12, 100], "the next step counts from the excluded total");
    assert_excluded_as_gap(&ingested, CHILD);

    // A lowered first step with no usage of its own reports nothing to check: it opens a new
    // epoch, and the next step that reports usage is checked from it (review D1 on PR #16).
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 60, 8, 0, 0), counter(4, 70, 10, 10, 2)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let mut totals = counted_totals(&ingested);
    totals.sort_unstable();
    assert_eq!(totals, [12, 100], "only the next step's own usage is counted");
    assert_eq!(reset_diagnostics(&ingested, CHILD), 1);
    assert!(boundary_diagnostics(&ingested).is_empty(), "no usage was left unverified");
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
}

#[test]
fn a_first_step_that_zeroes_the_inherited_total_adds_no_request() {
    // Codex's context-window fill lowers every measured category to zero; as a child's first
    // own step it reports no usage, so it must not become a zero-usage request (review D2).
    let inherited = counter(1, 90, 10, 90, 10);
    let root = fork_root(
        Some(std::slice::from_ref(&inherited)),
        &[inherited.clone(), counter(3, 0, 0, 0, 0), counter(4, 12, 3, 12, 3)],
    );
    let ingested = codex_rollout::ingest_root(root.path()).expect("ingest synthetic fork");
    let child = select(&ingested, &[CHILD]);
    assert_eq!(
        (child.counted.requests, child.counted.tokens.total().expect("valid sum")),
        (1, Some(15)),
        "only the next step's 15 tokens are the child's own"
    );
    assert!(boundary_diagnostics(&ingested).is_empty());
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
}

/// A `thread_settings_applied` event naming `thread`.
fn settings(ordinal: u64, thread: &str) -> String {
    json!({"ordinal": ordinal, "timestamp": "2026-01-01T00:00:01Z", "type": "event_msg",
           "payload": {"type": "thread_settings_applied", "thread_id": thread}})
    .to_string()
}

/// A `response_item` that urollup reads no usage from, such as the compaction a Guardian
/// review starts from.
fn compaction_item(ordinal: u64) -> String {
    json!({"ordinal": ordinal, "type": "response_item",
           "payload": {"type": "compaction", "encrypted_content": "synthetic"}})
    .to_string()
}

/// A `compacted` line whose `latest_token_usage_record` names `thread`.
fn compacted(ordinal: u64, thread: &str, response: &str, input: u64, output: u64) -> String {
    let record: serde_json::Value =
        serde_json::from_str(&direct(ordinal, thread, response, input, output))
            .expect("synthetic direct record");
    json!({"ordinal": ordinal, "type": "compacted", "payload": {
        "message": "", "latest_token_usage_record": record["payload"]
    }})
    .to_string()
}

/// The `session_meta` fields of a paginated Guardian review subagent of `PARENT`.
fn guardian_meta(boundary: u64) -> serde_json::Value {
    json!({"session_id": PARENT, "parent_thread_id": PARENT, "history_mode": "paginated",
           "thread_source": "guardian_review", "source": {"subagent": {"other": "guardian"}},
           "multi_agent_version": 2, "subagent_history_start_ordinal": boundary})
}

/// The parent rollout: one 100-token request, recorded and counted.
fn parent_records() -> [String; 2] {
    [direct(1, PARENT, "parent-response", 90, 10), counter(2, 90, 10, 90, 10)]
}

/// Ingests `root` with 1 and 8 workers, in discovery and reversed order, and returns the
/// first result after checking that every run built the same ledger.
fn ingest_every_way(root: &tempfile::TempDir) -> Ingested {
    let roots = codex_rollout::rollout_roots(&[root.path().to_owned()]);
    let mut baseline: Option<Ingested> = None;
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
            .expect("ingest synthetic fork");
            match &baseline {
                Some(expected) => assert_eq!(
                    ingested.ledger, expected.ledger,
                    "workers={workers}, reversed={reversed}"
                ),
                None => baseline = Some(ingested),
            }
        }
    }
    baseline.expect("at least one run")
}

/// The counted requests and tokens of one thread, and its coverage.
fn own_usage(ingested: &Ingested, native: &str) -> (u64, Option<u64>, Completeness) {
    let totals = select(ingested, &[native]);
    (
        totals.counted.requests,
        totals.counted.tokens.total().expect("valid sum"),
        totals.completeness,
    )
}

/// Checks the parent's usage and the history-wide accounting shared by every fork case: the
/// parent counts 100 tokens once when its rollout is present, every counted request has
/// one owner, and coverage is complete.
fn assert_parent_and_complete(ingested: &Ingested, parent_present: bool) {
    let parent = if parent_present { (1, Some(100)) } else { (0, None) };
    let (requests, tokens, _) = own_usage(ingested, PARENT);
    assert_eq!((requests, tokens), parent, "parent_present={parent_present}");
    assert!(
        ingested
            .ledger
            .requests
            .values()
            .filter(|request| request.counting == Counting::Counted)
            .all(|request| matches!(request.ownership, Ownership::Owned { .. })),
        "copies must not introduce conflicting owners"
    );
    assert!(boundary_diagnostics(ingested).is_empty());
    assert!(ingested.ledger.gaps.is_empty());
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
}

#[test]
fn a_migrated_guardians_own_records_before_a_past_end_boundary_are_its_own() {
    // Codex's legacy-to-paginated migration rewrites a subagent rollout in place and sets
    // subagent_history_start_ordinal one past the last line it migrated, so the Guardian's
    // own earlier turns all sit before the boundary. Their records name the Guardian. The
    // rollout starts with the parent's compaction item, or with a compacted line that keeps
    // no record (review A's probe d), which carries no usage and copies nothing.
    let record_free_compaction =
        json!({"ordinal": 1, "type": "compacted", "payload": {"message": ""}}).to_string();
    for (parent_present, first) in [false, true].into_iter().flat_map(|present| {
        [(present, compaction_item(1)), (present, record_free_compaction.clone())]
    }) {
        let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
        write_child(
            &root,
            guardian_meta(10),
            &[
                first,
                settings(2, CHILD),
                direct(4, CHILD, "guardian-r1", 18, 2),
                counter(5, 18, 2, 18, 2),
                direct(7, CHILD, "guardian-r2", 10, 2),
                counter(8, 28, 4, 10, 2),
                counter(9, 28, 4, 10, 2),
            ],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (2, Some(32), Completeness::Complete));
        assert_parent_and_complete(&ingested, parent_present);
        let totals = ledger_totals(&ingested.ledger).expect("totals");
        assert_eq!(totals.copy_only.requests, 0, "a counter follows its own record's thread");
        assert_eq!(ingested.ledger.coverage.copies, 0, "nothing in the rollout is copied");
    }
}

#[test]
fn a_native_guardians_inherited_checkpoint_stays_copied_inside_its_boundary() {
    // A Guardian forked from an earlier reviewer's checkpoint persists that checkpoint
    // (a compacted line naming the earlier reviewer and its running total) before the
    // boundary, then writes its own settings event at the boundary.
    for parent_present in [false, true] {
        let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
        write_child(
            &root,
            guardian_meta(3),
            &[
                compacted(1, OTHER, "reviewer-response", 40, 4),
                counter(2, 40, 4, 40, 4),
                settings(3, CHILD),
                direct(4, CHILD, "guardian-r1", 18, 2),
                counter(5, 58, 6, 18, 2),
            ],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
        assert_parent_and_complete(&ingested, parent_present);
        // The checkpoint's counter reports the response of the record the compacted line
        // keeps, so both copy one request of the earlier reviewer, whose rollout is absent.
        let totals = ledger_totals(&ingested.ledger).expect("totals");
        assert_eq!(totals.copy_only.requests, 1, "one copied request of the earlier reviewer");
    }
}

#[test]
fn a_migrated_subagents_copied_prefix_before_a_past_end_boundary_stays_copied() {
    // A legacy subagent copied its parent's settings events and counters (and, for a user
    // fork, its records); migration then dropped the parent's session header. The copied
    // lines still name or follow the parent, and the child's own start at its settings.
    for copied_record in [false, true] {
        for parent_present in [false, true] {
            let mut records = vec![settings(1, PARENT)];
            if copied_record {
                records.push(direct(2, PARENT, "parent-response", 90, 10));
            }
            records.extend([
                counter(3, 90, 10, 90, 10),
                settings(4, CHILD),
                direct(5, CHILD, "child-response", 18, 2),
                counter(6, 108, 12, 18, 2),
                counter(7, 108, 12, 18, 2),
            ]);
            let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
            write_child(
                &root,
                json!({"parent_thread_id": PARENT, "history_mode": "paginated",
                       "thread_source": "subagent", "subagent_history_start_ordinal": 8}),
                &records,
            );
            let ingested = ingest_every_way(&root);
            assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
            assert_parent_and_complete(&ingested, parent_present);
            let copy_only = ledger_totals(&ingested.ledger).expect("totals").copy_only.requests;
            // A copied record keys its counter to the parent's response, which the parent's
            // rollout owns when present; a counter without one stays an unkeyed copy.
            let expected = match (copied_record, parent_present) {
                (true, true) => 0,
                (true, false) | (false, _) => 1,
            };
            assert_eq!(copy_only, expected, "record={copied_record}, parent={parent_present}");
        }
    }
}

#[test]
fn a_migrated_counter_only_child_counts_from_its_own_settings_before_the_boundary() {
    // A counter-only legacy child names itself in its settings event (0.152 and later), so
    // its own counters follow that event even before a migration's past-end boundary, and
    // its first step is still checked against the copied total. The copied counter before
    // the event sits in no turn: the parent's matching counter proves it a copy, and
    // without the parent it is excluded as a gap (round 2: only the region before the
    // event is judged, not the whole file).
    for parent_present in [false, true] {
        let parent = [counter(1, 90, 10, 90, 10)];
        let root = fork_root(parent_present.then_some(&parent[..]), &[]);
        write_child(
            &root,
            json!({"parent_thread_id": PARENT, "history_mode": "paginated",
                   "thread_source": "subagent", "subagent_history_start_ordinal": 5}),
            &[
                counter(1, 90, 10, 90, 10),
                settings(2, CHILD),
                counter(3, 108, 12, 18, 2),
                counter(4, 113, 13, 5, 1),
            ],
        );
        let ingested = ingest_every_way(&root);
        if parent_present {
            assert_eq!(own_usage(&ingested, CHILD), (2, Some(26), Completeness::Complete));
            assert_parent_and_complete(&ingested, parent_present);
        } else {
            assert_eq!(own_usage(&ingested, CHILD).0, 2, "the own steps still count");
            assert_excluded_as_gap(&ingested, CHILD);
            assert_eq!(boundary_diagnostics(&ingested)[0].occurrences, 1);
        }
    }
}

#[test]
fn counter_usage_before_a_boundary_no_line_reaches_without_an_own_marker_is_a_gap() {
    // Without a turn, a settings event or a record naming the child, counters before a
    // boundary that lies past every line may be the child's migrated usage or a copied
    // prefix, so they are excluded as a coverage gap rather than silently treated as the
    // parent's copies. A counter whose total the parent root reports is its copy, so only
    // the undecided step counts as an occurrence.
    for parent_present in [false, true] {
        let parent = [counter(1, 90, 10, 90, 10)];
        let root = fork_root(parent_present.then_some(&parent[..]), &[]);
        write_child(
            &root,
            json!({"parent_thread_id": PARENT, "history_mode": "paginated",
                   "thread_source": "subagent", "subagent_history_start_ordinal": 3}),
            &[counter(1, 90, 10, 90, 10), counter(2, 108, 12, 18, 2)],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 0, "unplaced usage never counts");
        assert_eq!(
            counted_totals(&ingested).iter().sum::<u64>(),
            if parent_present { 100 } else { 0 }
        );
        assert_excluded_as_gap(&ingested, CHILD);
        assert_eq!(
            boundary_diagnostics(&ingested)[0].occurrences,
            if parent_present { 1 } else { 2 }
        );
    }
}

#[test]
fn own_records_on_both_sides_of_a_resumed_migration_boundary_all_count() {
    // A migrated child resumed later appends its new records after the boundary.
    for parent_present in [false, true] {
        let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
        write_child(
            &root,
            json!({"parent_thread_id": PARENT, "history_mode": "paginated",
                   "thread_source": "subagent", "subagent_history_start_ordinal": 4}),
            &[
                settings(1, CHILD),
                direct(2, CHILD, "child-r1", 18, 2),
                counter(3, 18, 2, 18, 2),
                settings(4, CHILD),
                direct(5, CHILD, "child-r2", 10, 2),
                counter(6, 28, 4, 10, 2),
            ],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (2, Some(32), Completeness::Complete));
        assert_parent_and_complete(&ingested, parent_present);
        assert_eq!(ledger_totals(&ingested.ledger).expect("totals").copy_only.requests, 0);
    }
}

/// A `turn_context` for `turn`, or one without a `turn_id` (before `rust-v0.100.0`).
fn turn(ordinal: u64, turn: Option<&str>) -> String {
    let mut payload = json!({"model": "gpt-test", "effort": "low"});
    if let Some(turn) = turn {
        payload["turn_id"] = json!(turn);
    }
    json!({"ordinal": ordinal, "timestamp": "2026-01-01T00:00:01Z", "type": "turn_context",
           "payload": payload})
    .to_string()
}

/// The `session_meta` fields of a subagent of `PARENT` that Codex's legacy-to-paginated
/// migration rewrote: no parent header survives, and the boundary is `boundary`.
fn migrated_meta(boundary: u64) -> serde_json::Value {
    json!({"parent_thread_id": PARENT, "history_mode": "paginated",
           "thread_source": "subagent", "subagent_history_start_ordinal": boundary})
}

/// A root parent with one turn and one 100-token counter.
fn parent_with_turn() -> [String; 2] {
    [turn(1, Some("p-turn-1")), counter(2, 90, 10, 90, 10)]
}

/// The occurrences of the one `codex-history-boundary-unverified` diagnostic.
fn undecided_steps(ingested: &Ingested) -> u64 {
    let diagnostics = boundary_diagnostics(ingested);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    diagnostics[0].occurrences
}

/// A migrated child's lines, with what it should report.
struct MigratedShape {
    name: &'static str,
    boundary: u64,
    records: Vec<String>,
    /// The child's own requests and tokens when its parent root is present.
    own_when_present: (u64, Option<u64>),
    /// The gap's occurrences when the parent is absent.
    undecided_when_absent: u64,
}

#[test]
fn a_migrated_childs_prefix_is_decided_by_the_turns_its_parent_root_recorded() {
    // Codex's migration keeps turn_context lines, and a child's own turn IDs are its own,
    // so the copy ends at the first turn the parent root never recorded. Without the parent
    // nothing decides it, and the undecided steps are the gap's occurrences. Shapes are
    // review A's probes a (pure copy), b (copied then own) and g (no copied prefix).
    let copied = [turn(1, Some("p-turn-1")), counter(2, 90, 10, 90, 10)];
    let own = [turn(3, Some("c-turn-1")), counter(4, 108, 12, 18, 2)];
    let fresh = [
        turn(1, Some("c-turn-1")),
        counter(2, 18, 2, 18, 2),
        turn(3, Some("c-turn-2")),
        counter(4, 23, 3, 5, 1),
    ];
    let shapes = [
        MigratedShape {
            name: "pure copy",
            boundary: 3,
            records: copied.to_vec(),
            own_when_present: (0, None),
            undecided_when_absent: 1,
        },
        MigratedShape {
            name: "copied then own",
            boundary: 5,
            records: [copied.to_vec(), own.to_vec()].concat(),
            own_when_present: (1, Some(20)),
            undecided_when_absent: 2,
        },
        MigratedShape {
            name: "no copied prefix",
            boundary: 5,
            records: fresh.to_vec(),
            own_when_present: (2, Some(26)),
            undecided_when_absent: 2,
        },
    ];
    for shape in shapes {
        for parent_present in [false, true] {
            let root = fork_root(parent_present.then_some(&parent_with_turn()[..]), &[]);
            write_child(&root, migrated_meta(shape.boundary), &shape.records);
            let ingested = ingest_every_way(&root);
            let name = shape.name;
            if parent_present {
                let (requests, tokens, coverage) = own_usage(&ingested, CHILD);
                assert_eq!((requests, tokens), shape.own_when_present, "{name}");
                assert_eq!(coverage, Completeness::Complete, "{name}");
                assert_parent_and_complete(&ingested, parent_present);
            } else {
                assert_eq!(own_usage(&ingested, CHILD).0, 0, "{name}");
                assert_excluded_as_gap(&ingested, CHILD);
                assert_eq!(undecided_steps(&ingested), shape.undecided_when_absent, "{name}");
            }
        }
    }
}

#[test]
fn an_inferred_own_turn_is_still_checked_against_the_copied_total() {
    // The first own step after the parent's copied 100 tokens reports 35 tokens but
    // advances the total by 70, so neither is proven the child's.
    let root = fork_root(Some(&parent_with_turn()), &[]);
    write_child(
        &root,
        migrated_meta(5),
        &[
            turn(1, Some("p-turn-1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("c-turn-1")),
            counter(4, 150, 20, 30, 5),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD).0, 0);
    assert_eq!(counted_totals(&ingested), [100]);
    assert_excluded_as_gap(&ingested, CHILD);
    assert_eq!(undecided_steps(&ingested), 1);
}

#[test]
fn a_turn_without_an_id_leaves_a_migrated_prefix_undecided() {
    // Before rust-v0.100.0 a turn_context has no turn_id, so the parent's turns cannot
    // place the counter after it, and its total is not one the parent reported.
    let root = fork_root(Some(&parent_with_turn()), &[]);
    write_child(&root, migrated_meta(3), &[turn(1, None), counter(2, 18, 2, 18, 2)]);
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD).0, 0);
    assert_excluded_as_gap(&ingested, CHILD);
    assert_eq!(undecided_steps(&ingested), 1);
}

#[test]
fn a_migrated_prefix_without_turns_is_decided_only_by_the_parents_totals() {
    // Counters before any turn_context carry no turn evidence: one whose cumulative total
    // the parent root reported is that counter's copy (review B's R6, a prefix-only child),
    // and any other is undecided. In review A's probe c a settings event names the child
    // after an undecided own step, so the step before it is a gap, not a silent copy.
    let prefix_only = [counter(1, 90, 10, 90, 10)];
    let probe_c = [
        counter(1, 90, 10, 90, 10),
        counter(2, 108, 12, 18, 2),
        settings(3, CHILD),
        counter(4, 113, 13, 5, 1),
    ];
    for parent_present in [false, true] {
        let parent = [counter(1, 90, 10, 90, 10)];
        let root = fork_root(parent_present.then_some(&parent[..]), &[]);
        write_child(&root, migrated_meta(3), &prefix_only);
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 0);
        if parent_present {
            assert_parent_and_complete(&ingested, parent_present);
        } else {
            assert_excluded_as_gap(&ingested, CHILD);
            assert_eq!(undecided_steps(&ingested), 1);
        }

        let root = fork_root(parent_present.then_some(&parent[..]), &[]);
        write_child(&root, migrated_meta(5), &probe_c);
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 1, "the step after the settings counts");
        assert_eq!(own_usage(&ingested, CHILD).1, Some(6));
        assert_excluded_as_gap(&ingested, CHILD);
        assert_eq!(undecided_steps(&ingested), if parent_present { 1 } else { 2 });
    }
}

#[test]
fn a_resumed_migrated_childs_prefix_is_judged_by_its_own_region() {
    // Review B's R2e and R2f: a migrated counter-only child reopened later writes lines at
    // its boundary, which says nothing about the migrated lines before it. Their turns still
    // decide them, and without the parent they are a gap, never a silent copy.
    let prefix = [
        turn(1, Some("p-turn-1")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("c-turn-1")),
        counter(4, 190, 20, 100, 10),
    ];
    let reopenings =
        [vec![settings(5, CHILD)], vec![turn(5, Some("c-turn-2")), counter(6, 190, 20, 100, 10)]];
    for reopening in reopenings {
        for parent_present in [false, true] {
            let root = fork_root(parent_present.then_some(&parent_with_turn()[..]), &[]);
            write_child(&root, migrated_meta(5), &[prefix.to_vec(), reopening.clone()].concat());
            let ingested = ingest_every_way(&root);
            if parent_present {
                assert_eq!(own_usage(&ingested, CHILD), (1, Some(110), Completeness::Complete));
                assert_parent_and_complete(&ingested, parent_present);
            } else {
                assert_eq!(own_usage(&ingested, CHILD).0, 0);
                assert_excluded_as_gap(&ingested, CHILD);
                assert_eq!(undecided_steps(&ingested), 2);
            }
        }
    }
}

#[test]
fn counter_only_turns_in_a_child_with_usage_records_are_not_yet_counted() {
    // Review A's probe f: a migrated child's own counter-only turn before the boundary,
    // then a usage record after it. A fork or subagent with usage records is accounted from
    // its records alone, so the 20 counter-only tokens are not counted; uro-r8si tracks
    // extending the counter twin rule to children. This pins the current result.
    let root = fork_root(Some(&parent_with_turn()), &[]);
    write_child(
        &root,
        migrated_meta(5),
        &[
            turn(1, Some("p-turn-1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("c-turn-1")),
            counter(4, 108, 12, 18, 2),
            settings(5, CHILD),
            turn(6, Some("c-turn-2")),
            direct(7, CHILD, "child-r2", 5, 1),
            counter(8, 113, 13, 5, 1),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (1, Some(6), Completeness::Complete));
    assert_parent_and_complete(&ingested, true);
}

#[test]
fn a_native_childs_own_settings_at_its_boundary_follow_its_copied_prefix() {
    // Codex persists a native paginated child's inherited items at ordinals 1 through the
    // boundary minus 1, then writes the child's own thread_settings_applied
    // (codex-rs/thread-store/src/live_thread.rs:155-177 and
    // codex-rs/core/src/session/mod.rs:1704-1716 at rust-v0.162.1). The settings event
    // therefore never precedes the copied counters, so they stay the parent's.
    for parent_present in [false, true] {
        let parent = [counter(1, 90, 10, 90, 10)];
        let root = fork_root(parent_present.then_some(&parent[..]), &[]);
        write_child(
            &root,
            json!({"parent_thread_id": PARENT, "history_mode": "paginated",
                   "thread_source": "subagent", "subagent_history_start_ordinal": 2}),
            &[counter(1, 90, 10, 90, 10), settings(2, CHILD), counter(3, 108, 12, 18, 2)],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
        assert_parent_and_complete(&ingested, parent_present);
    }
}

/// A Guardian child that a bounded migration kept from its own compaction on: the
/// compacted line keeps `guardian-r1`, then one turn reports `guardian-r2` with the running
/// total `total` (input, output).
fn compaction_child(total: (u64, u64)) -> Vec<String> {
    vec![
        compacted(1, CHILD, "guardian-r1", 40, 4),
        turn(2, Some("g-turn-2")),
        direct(3, CHILD, "guardian-r2", 18, 2),
        counter(4, total.0, total.1, 18, 2),
    ]
}

#[test]
fn a_compacted_record_naming_its_own_thread_counts_once_when_its_line_is_gone() {
    // A bounded migration keeps only the suffix from the child's newest compaction, so the
    // compacted line's record is the only one left of its response: it counts as the
    // child's own. A running total beyond what the child's records report shows responses
    // the migration dropped, which is a gap rather than a silent loss.
    for parent_present in [false, true] {
        let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
        write_child(&root, guardian_meta(5), &compaction_child((58, 6)));
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (2, Some(64), Completeness::Complete));
        assert_parent_and_complete(&ingested, parent_present);
        assert_eq!(ledger_totals(&ingested.ledger).expect("totals").copy_only.requests, 0);

        let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
        write_child(&root, guardian_meta(5), &compaction_child((158, 16)));
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 2, "both surviving responses count");
        assert_eq!(own_usage(&ingested, CHILD).1, Some(64));
        assert_excluded_as_gap(&ingested, CHILD);
        assert_eq!(undecided_steps(&ingested), 1);
        // A total that cannot be read cannot show that nothing was dropped either.
        let mut records = compaction_child((158, 16));
        let mut total: serde_json::Value =
            serde_json::from_str(&records[3]).expect("synthetic counter");
        total["payload"]["info"]["total_token_usage"]["cached_input_tokens"] = json!(200);
        records[3] = total.to_string();
        let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
        write_child(&root, guardian_meta(5), &records);
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).1, Some(64));
        assert_excluded_as_gap(&ingested, CHILD);
        assert_eq!(undecided_steps(&ingested), 1);
    }
}

#[test]
fn a_compacted_record_never_doubles_an_original_of_its_response() {
    // In the same rollout the compacted line copies the record before it; in another
    // rollout of the same thread both are originals of one response key, which merge.
    let same_file = fork_root(Some(&parent_records()), &[]);
    write_child(
        &same_file,
        guardian_meta(6),
        &[
            direct(1, CHILD, "guardian-r1", 40, 4),
            counter(2, 40, 4, 40, 4),
            compacted(3, CHILD, "guardian-r1", 40, 4),
            direct(4, CHILD, "guardian-r2", 18, 2),
            counter(5, 58, 6, 18, 2),
        ],
    );
    let other_file = fork_root(Some(&parent_records()), &[]);
    write_child(&other_file, guardian_meta(5), &compaction_child((58, 6)));
    let meta = json!({"ordinal": 0, "type": "session_meta",
                      "payload": {"id": CHILD, "parent_thread_id": PARENT}});
    fs::write(
        other_file
            .path()
            .join("sessions")
            .join(format!("rollout-2026-01-01T00-00-03-{CHILD}_{GRANDCHILD}.jsonl")),
        format!("{meta}\n{}\n", direct(1, CHILD, "guardian-r1", 40, 4)),
    )
    .expect("another rollout of the child");
    for root in [same_file, other_file] {
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (2, Some(64), Completeness::Complete));
        assert_parent_and_complete(&ingested, true);
    }
}

#[test]
fn a_parent_with_lineage_never_decides_a_childs_turns() {
    // Review C's r1. Before Codex wrote a top-level parent_thread_id, a spawned subagent
    // named its parent only in source.subagent.thread_spawn, so PARENT below looks like a
    // root. Its bounded migration kept only the suffix from its own compaction, so it no
    // longer records the grandparent turns (g1, g2) or its own earlier turn (p1) that
    // CHILD's copy holds. GRANDCHILD plays the grandparent root here. In the second shape
    // PARENT declares no boundary and records only a turn after CHILD's spawn, so nothing
    // in CHILD contradicts an inference from it: the nested link alone must exclude it.
    // Reading the link as PARENT's parent is uro-3b12.
    let spawned = json!({"source": {"subagent": {"thread_spawn": {
        "parent_thread_id": GRANDCHILD, "depth": 1}}}});
    let mut migrated = spawned.clone();
    migrated["history_mode"] = json!("paginated");
    migrated["subagent_history_start_ordinal"] = json!(4);
    let bounded = vec![
        json!({"ordinal": 1, "type": "compacted", "payload": {"message": ""}}).to_string(),
        turn(2, Some("p2")),
        counter(3, 270, 30, 45, 5),
    ];
    let later_turn_only = vec![turn(1, Some("p3")), counter(2, 45, 5, 45, 5)];
    for (parent_meta, parent_lines) in [(migrated, bounded), (spawned, later_turn_only)] {
        let root = tempfile::tempdir().expect("temporary log root");
        fs::create_dir(root.path().join("sessions")).expect("sessions directory");
        write_rollout(
            &root,
            GRANDCHILD,
            1,
            json!({}),
            &[
                turn(1, Some("g1")),
                counter(2, 90, 10, 90, 10),
                turn(3, Some("g2")),
                counter(4, 180, 20, 90, 10),
            ],
        );
        write_rollout(&root, PARENT, 2, parent_meta, &parent_lines);
        write_rollout(
            &root,
            CHILD,
            3,
            migrated_meta(11),
            &[
                turn(1, Some("g1")),
                counter(2, 90, 10, 90, 10),
                turn(3, Some("g2")),
                counter(4, 180, 20, 90, 10),
                turn(5, Some("p1")),
                counter(6, 225, 25, 45, 5),
                turn(7, Some("p2")),
                counter(8, 270, 30, 45, 5),
                turn(9, Some("c1")),
                counter(10, 297, 33, 27, 3),
            ],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, GRANDCHILD).1, Some(200), "the grandparent counts its own");
        // Real usage is 330 (200 + PARENT's 100 + CHILD's 30); G's 200 must never count twice.
        let counted: u64 = counted_totals(&ingested).iter().sum();
        assert!(counted <= 330, "counted {counted}");
        assert_eq!(own_usage(&ingested, CHILD).0, 0, "nothing proves CHILD's own lines");
        assert_eq!(own_usage(&ingested, CHILD).2, partial());
    }
}

/// Partial coverage from an unverified-boundary gap.
fn partial() -> Completeness {
    Completeness::Partial(BTreeSet::from([PartialReason::UnobservedGap]))
}

#[test]
fn evidence_against_an_inferred_own_start_turns_its_region_into_a_gap() {
    // Review C3: a child's own lines never precede a turn or a total its parent recorded.
    // Here the parent root lost turn p1 (as a rollback or bounded migration can), so the
    // child's copy of p1 looks like its own start, but the parent's turn p2 after it, or
    // the parent's total 200, proves it copied. Everything from p1 to the boundary is then
    // undecided, never counted: the parent's 200 tokens already include p1's 50.
    let parent = [
        turn(1, Some("p0")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("p2")),
        counter(4, 180, 20, 45, 5),
    ];
    let later_turn = vec![
        turn(1, Some("p0")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("p1")),
        counter(4, 135, 15, 45, 5),
        turn(5, Some("p2")),
        counter(6, 180, 20, 45, 5),
        turn(7, Some("c1")),
        counter(8, 207, 23, 27, 3),
    ];
    let later_total = vec![
        turn(1, Some("p0")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("p1")),
        counter(4, 135, 15, 45, 5),
        counter(5, 180, 20, 45, 5),
        counter(6, 207, 23, 27, 3),
    ];
    for (records, boundary) in [(later_turn, 9), (later_total, 7)] {
        let root = fork_root(Some(&parent), &[]);
        write_child(&root, migrated_meta(boundary), &records);
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 0, "the contradicted region never counts");
        assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 200);
        assert_excluded_as_gap(&ingested, CHILD);
        // The p1 step counted before the contradiction, and the own-looking c1 step after
        // it; the parent's 200 is a copy.
        assert_eq!(undecided_steps(&ingested), 2);
    }
}

#[test]
fn a_rolled_back_spawn_turn_is_still_counted_twice_until_uro_eh0d() {
    // Review C's r2n. Codex's migration of a root drops a rolled-back turn's turn_context
    // and counters, so the root's next counter step carries that turn's usage in its
    // delta, while a subagent spawned in that turn still holds its copy and infers its own
    // start there. Real usage is 330; this pins today's 380 and complete coverage, which
    // uro-eh0d tracks (main also double counts this for an unmigrated child).
    let parent = [
        turn(1, Some("t1")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("t2")),
        counter(4, 180, 20, 90, 10),
        turn(6, Some("t4")),
        counter(7, 270, 30, 45, 5),
    ];
    let root = fork_root(Some(&parent), &[]);
    write_child(
        &root,
        migrated_meta(9),
        &[
            turn(1, Some("t1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("t2")),
            counter(4, 180, 20, 90, 10),
            turn(5, Some("t3")),
            counter(6, 225, 25, 45, 5),
            turn(7, Some("c1")),
            counter(8, 252, 28, 27, 3),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (2, Some(80), Completeness::Complete));
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 380);
}

#[test]
fn a_copied_total_matches_its_parents_whether_or_not_a_count_is_written() {
    // Review C4's r8: Codex's migration re-serializes a legacy counter with
    // cache_write_input_tokens: 0, which the unmigrated parent's original omits. A missing
    // count reads as 0, so the copy still matches its parent's total.
    let mut migrated_copy: serde_json::Value =
        serde_json::from_str(&counter(1, 90, 10, 90, 10)).expect("synthetic counter");
    for usage in ["total_token_usage", "last_token_usage"] {
        migrated_copy["payload"]["info"][usage]["cache_write_input_tokens"] = json!(0);
    }
    let root = fork_root(Some(&[counter(1, 90, 10, 90, 10)]), &[]);
    write_child(&root, migrated_meta(3), &[migrated_copy.to_string()]);
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD).0, 0);
    assert_parent_and_complete(&ingested, true);
}

#[test]
fn a_seeded_bounded_child_with_records_reports_its_seed_as_a_gap() {
    // Review C5's r3: a child seeded with its parent's 100-token total, migrated from its
    // own compaction, dropped nothing, yet its first running total exceeds what its own
    // records report by the seed. Its usage counts, and the check conservatively reports a
    // gap; discounting a seed needs the parent's totals, which a root with usage records
    // does not keep.
    let root = fork_root(Some(&parent_records()), &[]);
    write_child(
        &root,
        json!({"parent_thread_id": PARENT, "history_mode": "paginated",
               "thread_source": "subagent", "subagent_history_start_ordinal": 5}),
        &[
            compacted(1, CHILD, "child-r1", 18, 2),
            turn(2, Some("c2")),
            direct(3, CHILD, "child-r2", 9, 1),
            counter(4, 117, 13, 9, 1),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD).1, Some(30));
    assert_excluded_as_gap(&ingested, CHILD);
    assert_eq!(undecided_steps(&ingested), 1);
}

#[test]
fn a_counter_after_a_compacted_original_in_a_counter_only_rollout_is_its_twin() {
    // Review C6's r7: no Codex writer is known for it, but a counter that reports the
    // response a self-naming compacted original already counts must not count it again.
    for parent_present in [false, true] {
        let root = fork_root(parent_present.then_some(&parent_records()[..]), &[]);
        write_child(
            &root,
            guardian_meta(4),
            &[
                compacted(1, CHILD, "guardian-r1", 18, 2),
                turn(2, Some("g-turn-2")),
                counter(3, 18, 2, 18, 2),
            ],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
        assert_parent_and_complete(&ingested, parent_present);
    }
}

#[test]
fn an_inferred_own_start_stays_voidable_until_the_boundary() {
    // Review D1. The parent root lost turns x and y, so the child's copy of x looks like its
    // own start, and the parent's recorded t2 later proves it copied. A copied line naming
    // the parent, a thread-less settings event before another lost turn, or a line without
    // an ordinal must not close or move that start first. Real usage is 280: the parent's
    // 250 includes x and y, plus the child's 30.
    let parent = [
        turn(1, Some("t1")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("t2")),
        counter(4, 225, 25, 45, 5),
    ];
    let thread_less = json!({"ordinal": 5, "type": "event_msg",
        "payload": {"type": "thread_settings_applied"}})
    .to_string();
    let mut no_ordinal: serde_json::Value =
        serde_json::from_str(&turn(0, Some("z"))).expect("synthetic turn");
    no_ordinal.as_object_mut().expect("line").remove("ordinal");
    let shapes = [
        // d2: a copied settings event naming the parent
        vec![
            turn(1, Some("t1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("x")),
            counter(4, 135, 15, 45, 5),
            counter(5, 180, 20, 45, 5),
            settings(6, PARENT),
            turn(8, Some("t2")),
            counter(9, 225, 25, 45, 5),
            turn(10, Some("c1")),
            counter(11, 252, 28, 27, 3),
        ],
        // d1: a thread-less settings event, then another lost turn
        vec![
            turn(1, Some("t1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("x")),
            counter(4, 135, 15, 45, 5),
            thread_less,
            turn(6, Some("y")),
            counter(7, 180, 20, 45, 5),
            turn(8, Some("t2")),
            counter(9, 225, 25, 45, 5),
            turn(10, Some("c1")),
            counter(11, 252, 28, 27, 3),
        ],
        // d4: a turn_context without an ordinal
        vec![
            turn(1, Some("t1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("x")),
            counter(4, 135, 15, 45, 5),
            counter(5, 180, 20, 45, 5),
            no_ordinal.to_string(),
            turn(8, Some("t2")),
            counter(9, 225, 25, 45, 5),
            turn(10, Some("c1")),
            counter(11, 252, 28, 27, 3),
        ],
    ];
    for records in shapes {
        let root = fork_root(Some(&parent), &[]);
        write_child(&root, migrated_meta(13), &records);
        let ingested = ingest_every_way(&root);
        assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 250);
        assert_eq!(own_usage(&ingested, CHILD).0, 0, "the contradicted region never counts");
        assert_excluded_as_gap(&ingested, CHILD);
    }
}

#[test]
fn a_nested_spawn_link_alone_leaves_a_rollouts_counters_to_the_root_rules() {
    // Review D2. An unmigrated old-format subagent names its parent only in
    // source.subagent.thread_spawn and holds no copy: no boundary and no foreign header.
    // A 0.153 or 0.154 resume added a usage record beside its legacy counter-only response,
    // and both responses are its own: 100 + 30. The link keeps it from deciding a child's
    // turns, but its own counters keep the root rules, as on main.
    let root = tempfile::tempdir().expect("temporary log root");
    fs::create_dir(root.path().join("sessions")).expect("sessions directory");
    write_rollout(&root, PARENT, 1, json!({}), &[turn(1, Some("p1")), counter(2, 45, 5, 45, 5)]);
    write_rollout(
        &root,
        CHILD,
        2,
        json!({"source": {"subagent": {"thread_spawn": {"parent_thread_id": PARENT, "depth": 1}}}}),
        &[
            turn(1, Some("c1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("c2")),
            direct(4, CHILD, "child-r2", 27, 3),
            counter(5, 117, 13, 27, 3),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (2, Some(130), Completeness::Complete));
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 180);
}

#[test]
fn a_compacted_twin_matches_whether_or_not_a_count_is_written() {
    // Review D3's d5: the counter writes cache_write_input_tokens: 0, which the compacted
    // record omits; a missing count reads as 0, so the counter is still the record's twin.
    let mut twin: serde_json::Value =
        serde_json::from_str(&counter(3, 18, 2, 18, 2)).expect("synthetic counter");
    for usage in ["total_token_usage", "last_token_usage"] {
        twin["payload"]["info"][usage]["cache_write_input_tokens"] = json!(0);
    }
    let root = fork_root(Some(&parent_records()), &[]);
    write_child(
        &root,
        guardian_meta(4),
        &[compacted(1, CHILD, "guardian-r1", 18, 2), turn(2, Some("g-turn-2")), twin.to_string()],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
    assert_parent_and_complete(&ingested, true);
}

#[test]
fn a_line_naming_the_child_itself_never_voids_an_inferred_start() {
    // Review E2. A child inferred its start at c-turn-1, then a settings event names the
    // child before the boundary, as a later release's resume writes. That line is not
    // evidence against the start: only a line naming another thread is. It ends the window,
    // because the lines after it are the child's by name.
    let root = fork_root(Some(&parent_with_turn()), &[]);
    write_child(
        &root,
        migrated_meta(9),
        &[
            turn(1, Some("p-turn-1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("c-turn-1")),
            counter(4, 108, 12, 18, 2),
            settings(5, CHILD),
            turn(6, Some("c-turn-2")),
            counter(7, 113, 13, 5, 1),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (2, Some(26), Completeness::Complete));
    assert_parent_and_complete(&ingested, true);
}

/// The `session_meta` fields of a legacy user fork of `PARENT` that Codex's
/// legacy-to-paginated migration rewrote (`uro-p9ua`): paginated, keeping its fork origin,
/// with no boundary, no `history_base` and none of its copied prefix's headers.
fn migrated_fork_meta() -> serde_json::Value {
    json!({"forked_from_id": PARENT, "history_mode": "paginated", "source": "cli",
           "thread_source": "user"})
}

/// A root parent with two turns: 100 tokens, then 60, for a running total of 160.
fn parent_with_two_turns() -> [String; 4] {
    [
        turn(1, Some("p-turn-1")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("p-turn-2")),
        counter(4, 140, 20, 50, 10),
    ]
}

/// A migrated fork's lines: its copy of `parent_with_two_turns`, then `own`.
fn migrated_fork(own: &[String]) -> Vec<String> {
    [parent_with_two_turns().to_vec(), own.to_vec()].concat()
}

/// The fork's own first turn: 20 tokens on the copied 160.
fn fork_first_turn() -> [String; 2] {
    [turn(5, Some("f-turn-1")), counter(6, 158, 22, 18, 2)]
}

/// Checks the history-wide accounting of a migrated fork whose parent is present: the
/// parent counts its 160 tokens once, every counted request has one owner, and coverage is
/// complete.
fn assert_fork_parent_and_complete(ingested: &Ingested) {
    let (requests, tokens, _) = own_usage(ingested, PARENT);
    assert_eq!((requests, tokens), (2, Some(160)));
    assert!(
        ingested
            .ledger
            .requests
            .values()
            .filter(|request| request.counting == Counting::Counted)
            .all(|request| matches!(request.ownership, Ownership::Owned { .. })),
        "copies must not introduce conflicting owners"
    );
    assert!(boundary_diagnostics(ingested).is_empty());
    assert!(ingested.ledger.gaps.is_empty());
    assert_eq!(
        ledger_totals(&ingested.ledger).expect("totals").completeness,
        Completeness::Complete
    );
}

#[test]
fn a_migrated_user_forks_copied_prefix_counts_once() {
    // Review B3 of PR #32 (repro R4). Codex's migration drops the copied prefix's header and
    // gives a user fork no boundary, so the copied counters look like the fork's own: main
    // reports 340 with complete coverage. The parent root recorded p-turn-1 and p-turn-2, so
    // the copy ends at f-turn-1, whose first step continues the copied 160 by its own 20.
    let root = fork_root(Some(&parent_with_two_turns()), &[]);
    write_child(&root, migrated_fork_meta(), &migrated_fork(&fork_first_turn()));
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
    assert_fork_parent_and_complete(&ingested);
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 180);
    // Two copied counters and one copied region, as for the same fork before migration.
    assert_eq!(ingested.ledger.coverage.copies, 3);
}

#[test]
fn a_migrated_user_fork_whose_parent_cannot_decide_its_prefix_is_a_gap() {
    // Without the parent root, or with a parent whose rollout may lack turns the fork
    // copied (here an old-format subagent's nested spawn link), nothing decides which lines
    // are the fork's own, so all three steps are excluded as a gap rather than counted.
    let spawned = json!({"source": {"subagent": {"thread_spawn": {
        "parent_thread_id": GRANDCHILD, "depth": 1}}}});
    for parent in [None, Some(spawned)] {
        let root = tempfile::tempdir().expect("temporary log root");
        fs::create_dir(root.path().join("sessions")).expect("sessions directory");
        if let Some(meta) = &parent {
            write_rollout(&root, PARENT, 1, meta.clone(), &parent_with_two_turns());
        }
        write_child(&root, migrated_fork_meta(), &migrated_fork(&fork_first_turn()));
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 0, "parent={parent:?}");
        assert_eq!(
            counted_totals(&ingested).iter().sum::<u64>(),
            if parent.is_some() { 160 } else { 0 }
        );
        assert_excluded_as_gap(&ingested, CHILD);
        assert_eq!(undecided_steps(&ingested), 3);
    }
}

#[test]
fn a_migrated_forks_own_turns_after_its_copy_count_from_the_copied_total() {
    // Its own steps after the first count as deltas, and a settings event naming the fork,
    // as a 0.152 or later resume writes, keeps the lines after it the fork's own.
    let root = fork_root(Some(&parent_with_two_turns()), &[]);
    let own = [
        fork_first_turn().to_vec(),
        vec![settings(7, CHILD), turn(8, Some("f-turn-2")), counter(9, 170, 25, 12, 3)],
    ]
    .concat();
    write_child(&root, migrated_fork_meta(), &migrated_fork(&own));
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (2, Some(35), Completeness::Complete));
    assert_fork_parent_and_complete(&ingested);
}

#[test]
fn a_migrated_forks_first_own_step_is_still_checked_against_the_copied_total() {
    // f-turn-1 reports 20 tokens but advances the copied 160 by 70, so neither is proven the
    // fork's own; the next step still counts from its total.
    let root = fork_root(Some(&parent_with_two_turns()), &[]);
    let own = [
        turn(5, Some("f-turn-1")),
        counter(6, 200, 30, 18, 2),
        turn(7, Some("f-turn-2")),
        counter(8, 212, 33, 12, 3),
    ];
    write_child(&root, migrated_fork_meta(), &migrated_fork(&own));
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD).0, 1);
    assert_eq!(own_usage(&ingested, CHILD).1, Some(15));
    assert_excluded_as_gap(&ingested, CHILD);
    assert_eq!(undecided_steps(&ingested), 1);
}

#[test]
fn evidence_against_a_migrated_forks_inferred_start_turns_the_rest_into_a_gap() {
    // The parent root lost turn x (as a rollback can), so the fork's copy of x looks like
    // its own start, and the parent's p-turn-2 after it proves it copied. A fork has no
    // boundary to close the window, so everything from x on is undecided, never counted.
    let parent = [
        turn(1, Some("p-turn-1")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("p-turn-2")),
        counter(4, 180, 20, 45, 5),
    ];
    let root = fork_root(Some(&parent), &[]);
    write_child(
        &root,
        migrated_fork_meta(),
        &[
            turn(1, Some("p-turn-1")),
            counter(2, 90, 10, 90, 10),
            turn(3, Some("x")),
            counter(4, 135, 15, 45, 5),
            turn(5, Some("p-turn-2")),
            counter(6, 180, 20, 45, 5),
            turn(7, Some("f-turn-1")),
            counter(8, 207, 23, 27, 3),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD).0, 0, "the contradicted region never counts");
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 200);
    assert_excluded_as_gap(&ingested, CHILD);
    assert_eq!(undecided_steps(&ingested), 2);
}

#[test]
fn a_native_user_fork_with_usage_records_is_untouched() {
    // Codex 0.153 and later write a referenced paginated fork: a session_meta at its
    // history_base end, its own settings event, then its own records and counters
    // (codex-rs/core/src/session/mod.rs:1692-1703 at rust-v0.162.1). Its first counter is
    // seeded with the parent's 160, and its record names it.
    let root = fork_root(Some(&parent_with_two_turns()), &[]);
    let meta = json!({"ordinal": 5, "type": "session_meta", "payload": {
        "id": CHILD, "forked_from_id": PARENT, "forked_from_ordinal_exclusive": 5,
        "history_mode": "paginated", "source": "cli", "thread_source": "user",
        "history_base": {"thread_id": PARENT, "end_ordinal_exclusive": 5, "end_byte_offset": 900}
    }});
    let lines = [
        meta.to_string(),
        settings(6, CHILD),
        turn(7, Some("f-turn-1")),
        direct(8, CHILD, "child-r1", 18, 2),
        counter(9, 158, 22, 18, 2),
    ];
    fs::write(
        root.path().join("sessions").join(format!("rollout-2026-01-01T00-00-02-{CHILD}.jsonl")),
        format!("{}\n", lines.join("\n")),
    )
    .expect("native fork");
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
    assert_fork_parent_and_complete(&ingested);
}

#[test]
fn a_counter_only_native_fork_still_counts_its_seeded_total_until_uro_p12c() {
    // Pins main's behavior for a counter-only referenced fork (Codex 0.150 to 0.152), which
    // the migrated-fork inference leaves untouched: a history_base, or a first counter whose
    // total its own usage does not explain, marks a fork that copied nothing. Codex seeds
    // its running total with the parent's (codex-rs/core/src/session/mod.rs:1683-1688 at
    // rust-v0.162.1), and a fork with neither a boundary nor copied counters starts from
    // zero, so its first step counts the parent's 160 again: 180 instead of 20, which
    // uro-p12c tracks.
    let history_base = json!({"thread_id": PARENT, "end_ordinal_exclusive": 5,
                              "end_byte_offset": 900});
    for declared in [true, false] {
        let mut meta = migrated_fork_meta();
        if declared {
            meta["history_base"] = history_base.clone();
        }
        let root = fork_root(Some(&parent_with_two_turns()), &[]);
        write_child(&root, meta, &fork_first_turn());
        let ingested = ingest_every_way(&root);
        assert_eq!(
            own_usage(&ingested, CHILD),
            (1, Some(180), Completeness::Complete),
            "history_base={declared}"
        );
        assert!(boundary_diagnostics(&ingested).is_empty());
    }
}

#[test]
fn a_child_that_copies_nothing_counts_its_own_lines_without_its_parent() {
    // Children that hold no copy keep counting from zero, with or without their parent:
    // - unmigrated legacy rollouts, an old-format subagent and a fork origin without a
    //   header (an unmigrated copy keeps its prefix's header);
    // - native paginated subagents with no inherited context, named by thread_source or
    //   only by a subagent source: Codex gives such a child no boundary
    //   (codex-rs/thread-store/src/live_thread.rs:151-177 at rust-v0.162.1) while its
    //   migration gives one to every subagent by source or thread_source
    //   (rollout_migration.rs:458-473 and 729-731);
    // - a referenced fork with a history_base whose parent had no counter to seed it from;
    // - a referenced fork from a thread's start, which has no history_base but names itself
    //   in its settings event first.
    let own = [
        turn(2, Some("c-turn-1")),
        counter(3, 18, 2, 18, 2),
        turn(4, Some("c-turn-2")),
        counter(5, 23, 3, 5, 1),
    ];
    let legacy_subagent = (
        json!({"parent_thread_id": PARENT, "source": {"subagent": {"thread_spawn": {
            "parent_thread_id": PARENT, "depth": 1}}}}),
        None,
    );
    let legacy_fork = (json!({"forked_from_id": PARENT, "source": "cli"}), None);
    let native_subagent = (
        json!({"parent_thread_id": PARENT, "history_mode": "paginated",
               "thread_source": "subagent"}),
        None,
    );
    let review_subagent = (
        json!({"parent_thread_id": PARENT, "history_mode": "paginated",
               "source": {"subagent": "review"}}),
        None,
    );
    let mut referenced = migrated_fork_meta();
    referenced["history_base"] =
        json!({"thread_id": PARENT, "end_ordinal_exclusive": 1, "end_byte_offset": 120});
    let unseeded_referenced_fork = (referenced, None);
    let fork_from_start = (migrated_fork_meta(), Some(settings(1, CHILD)));
    for (meta, first) in [
        legacy_subagent,
        legacy_fork,
        native_subagent,
        review_subagent,
        unseeded_referenced_fork,
        fork_from_start,
    ] {
        for parent_present in [false, true] {
            let root = fork_root(parent_present.then_some(&parent_with_two_turns()[..]), &[]);
            let records = [first.clone().into_iter().collect::<Vec<_>>(), own.to_vec()].concat();
            write_child(&root, meta.clone(), &records);
            let ingested = ingest_every_way(&root);
            assert_eq!(
                own_usage(&ingested, CHILD),
                (2, Some(26), Completeness::Complete),
                "{meta}, parent={parent_present}"
            );
            assert!(boundary_diagnostics(&ingested).is_empty());
            assert_eq!(
                ledger_totals(&ingested.ledger).expect("totals").completeness,
                Completeness::Complete
            );
        }
    }
}

#[test]
fn a_copied_first_counter_that_carries_a_rolled_back_turn_is_not_a_seed() {
    // The parent rolled back its first turn before the fork. Codex's migration replays both
    // rollouts through rollback planning, which drops that turn's turn_context and counters,
    // so the first counter left in each carries its usage: 160 on a last usage of 60. In the
    // fork that counter is the copy of a total its parent root reported, not a seed, so the
    // fork's copy still ends at f-turn-1.
    let parent = [turn(3, Some("p-turn-2")), counter(4, 140, 20, 50, 10)];
    let root = fork_root(Some(&parent), &[]);
    write_child(
        &root,
        migrated_fork_meta(),
        &[parent.to_vec(), fork_first_turn().to_vec()].concat(),
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (1, Some(20), Completeness::Complete));
    assert_eq!(own_usage(&ingested, PARENT), (1, Some(160), Completeness::Complete));
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 180);
    assert!(boundary_diagnostics(&ingested).is_empty());
}

#[test]
fn a_parent_rollback_after_a_fork_is_still_counted_twice_until_uro_eh0d() {
    // Review A1 on PR #33 (probe P1). A user forks after the parent's p-turn-2 (60), then the
    // parent rolls p-turn-2 back and runs p-turn-3 (40). The parent's migration drops the
    // rolled-back turn's turn_context and counter, so its next step carries both turns'
    // usage, while the fork still holds its copy of p-turn-2. The parent never recorded that
    // turn, so the fork's own start is inferred there, and its first step passes the check.
    // Real usage is 220; this pins today's 280 and complete coverage, which uro-eh0d tracks
    // (main reports 380).
    let parent = [
        turn(1, Some("p-turn-1")),
        counter(2, 90, 10, 90, 10),
        turn(3, Some("p-turn-3")),
        counter(4, 175, 25, 35, 5),
    ];
    let root = fork_root(Some(&parent), &[]);
    write_child(&root, migrated_fork_meta(), &migrated_fork(&fork_first_turn()));
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (2, Some(80), Completeness::Complete));
    assert_eq!(own_usage(&ingested, PARENT), (2, Some(200), Completeness::Complete));
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 280);
}

#[test]
fn a_rolled_back_first_turn_under_a_parent_with_usage_records_is_still_counted_twice_until_uro_eh0d()
 {
    // Review A2 on PR #33 (probe P3b). The parent rolled back its first turn, so its first
    // migrated counter carries that turn's usage (160 on a last usage of 60), and a fork
    // copied that counter. Then a release from 0.153 resumed the parent, so it holds usage
    // records, and only counter-only roots supply the totals the seeded check compares: the
    // fork's copied first counter looks seeded, keeps main's rules, and counts the copy
    // again. Real usage is 190; this pins today's 350 and complete coverage, as on main,
    // which uro-eh0d tracks.
    let parent = [
        turn(1, Some("p-turn-2")),
        counter(2, 140, 20, 50, 10),
        turn(3, Some("p-turn-3")),
        direct(4, PARENT, "parent-r3", 9, 1),
        counter(5, 149, 21, 9, 1),
    ];
    let root = fork_root(Some(&parent), &[]);
    write_child(
        &root,
        migrated_fork_meta(),
        &[
            turn(1, Some("p-turn-2")),
            counter(2, 140, 20, 50, 10),
            turn(3, Some("f-turn-1")),
            counter(4, 158, 22, 18, 2),
        ],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD), (2, Some(180), Completeness::Complete));
    assert_eq!(own_usage(&ingested, PARENT).1, Some(170));
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 350);
}

#[test]
fn a_forks_own_step_that_repeats_a_parent_total_is_still_dropped_as_a_copy_until_uro_uwv0() {
    // Review A4 on PR #33 (probes P4b and P4c). The fork, taken after p-turn-1, retries the
    // parent's next prompt, and its response repeats the parent's p-turn-2 in every count.
    // Its total is then one the parent reported, which voids the inferred start, and that
    // counter is decided as the parent's copy, so the fork's 60 is dropped with complete
    // coverage. PR #32's rule does the same for a migrated subagent with a boundary past its
    // end. Real usage is 220; this pins 160 (main reports 320 for the fork).
    for meta in [migrated_fork_meta(), migrated_meta(9)] {
        let root = fork_root(Some(&parent_with_two_turns()), &[]);
        write_child(
            &root,
            meta.clone(),
            &[
                turn(1, Some("p-turn-1")),
                counter(2, 90, 10, 90, 10),
                turn(3, Some("f-turn-1")),
                counter(4, 140, 20, 50, 10),
            ],
        );
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (0, None, Completeness::Complete), "{meta}");
        assert_fork_parent_and_complete(&ingested);
        assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 160);
    }
}

#[test]
fn a_settings_event_without_a_thread_inside_an_open_start_keeps_the_childs_lines() {
    // Review A5 on PR #33 (probe P9). A settings event without a thread_id (written before
    // 0.152) names no owner, so between two counters of the child's first own turn it must
    // not hand the second back to the parent as a copy. This holds for an unmarked fork and,
    // through the same code, for PR #32's migrated subagent with a boundary past its end.
    // The child's own 20 and 15 both count: 195 in all, where 180 dropped the 15 silently.
    let thread_less = json!({"ordinal": 7, "type": "event_msg",
        "payload": {"type": "thread_settings_applied"}})
    .to_string();
    let own = [fork_first_turn().to_vec(), vec![thread_less, counter(8, 170, 25, 12, 3)]].concat();
    for meta in [migrated_fork_meta(), migrated_meta(9)] {
        let root = fork_root(Some(&parent_with_two_turns()), &[]);
        write_child(&root, meta.clone(), &migrated_fork(&own));
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (2, Some(35), Completeness::Complete), "{meta}");
        assert_fork_parent_and_complete(&ingested);
        assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 195);
    }
}

/// A `thread_settings_applied` snapshot selecting `tier`, naming `thread` when one is
/// given, and with an `ordinal` when one is given.
fn tier_settings(ordinal: Option<u64>, thread: Option<&str>, tier: &str) -> String {
    let mut line = json!({"timestamp": "2026-01-01T00:00:01Z", "type": "event_msg",
        "payload": {"type": "thread_settings_applied", "thread_settings": {"service_tier": tier}}});
    if let Some(ordinal) = ordinal {
        line["ordinal"] = json!(ordinal);
    }
    if let Some(thread) = thread {
        line["payload"]["thread_id"] = json!(thread);
    }
    line.to_string()
}

/// The tiers of the counted requests `native` owns.
fn owned_tiers(ingested: &Ingested, native: &str) -> Vec<Option<&'static str>> {
    let owner = thread(native);
    ingested
        .ledger
        .requests
        .values()
        .filter(|request| request.counting == Counting::Counted)
        .filter(
            |request| matches!(&request.ownership, Ownership::Owned { thread } if *thread == owner),
        )
        .map(|request| {
            request.pricing.and_then(|pricing| pricing.service_tier.map(|tier| tier.name.as_str()))
        })
        .collect()
}

#[test]
fn an_unmarked_forks_thread_less_settings_do_not_set_its_tier_until_uro_1uy7() {
    // Found merging PR #33 into PR #19. Every line of an unmarked fork falls before its
    // boundary, as every line of a migrated subagent with a boundary past its end does, so
    // a settings snapshot without a thread_id (written before 0.152) never sets the
    // child's tier, even inside its own open start: the rule cannot yet tell it from the
    // parent's copied settings. Nothing reads the tier yet. The child's own 20 and 15
    // still count, 195 in all, and the parent keeps its own tier. Both placements
    // uro-1uy7 will change are pinned: inside the open start, and before the fork's
    // first own turn, where no start is open yet.
    let inside = [
        fork_first_turn().to_vec(),
        vec![
            tier_settings(Some(7), None, "priority"),
            turn(8, Some("f-turn-2")),
            counter(9, 170, 25, 12, 3),
        ],
    ]
    .concat();
    let before = vec![
        tier_settings(Some(5), None, "priority"),
        turn(6, Some("f-turn-1")),
        counter(7, 158, 22, 18, 2),
        turn(8, Some("f-turn-2")),
        counter(9, 170, 25, 12, 3),
    ];
    let parent =
        [vec![tier_settings(None, Some(PARENT), "flex")], parent_with_two_turns().to_vec()]
            .concat();
    for (own, meta) in [&inside, &before]
        .into_iter()
        .flat_map(|own| [migrated_fork_meta(), migrated_meta(12)].map(|meta| (own, meta)))
    {
        let root = fork_root(Some(&parent), &[]);
        write_child(&root, meta.clone(), &migrated_fork(own));
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD), (2, Some(35), Completeness::Complete), "{meta}");
        assert_eq!(owned_tiers(&ingested, CHILD), [None, None], "{meta}");
        assert_fork_parent_and_complete(&ingested);
        assert_eq!(owned_tiers(&ingested, PARENT), [Some("flex"), Some("flex")], "{meta}");
        assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 195);
    }
}

#[test]
fn a_migrated_forks_counters_before_every_turn_are_decided_only_by_the_parents_totals() {
    // The unmarked counterpart of
    // a_migrated_prefix_without_turns_is_decided_only_by_the_parents_totals (review A6 on
    // PR #33): no turn places a counter before the fork's first turn_context, and no line
    // reaches an unmarked boundary, so such a counter is the parent's copy only when the
    // parent root reported its total, and is otherwise a gap, never a silent copy.
    let prefix_only = [counter(1, 90, 10, 90, 10)];
    let probe_c = [
        counter(1, 90, 10, 90, 10),
        counter(2, 108, 12, 18, 2),
        settings(3, CHILD),
        counter(4, 113, 13, 5, 1),
    ];
    for parent_present in [false, true] {
        let parent = [counter(1, 90, 10, 90, 10)];
        let root = fork_root(parent_present.then_some(&parent[..]), &[]);
        write_child(&root, migrated_fork_meta(), &prefix_only);
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 0);
        if parent_present {
            assert_parent_and_complete(&ingested, parent_present);
        } else {
            assert_excluded_as_gap(&ingested, CHILD);
            assert_eq!(undecided_steps(&ingested), 1);
        }

        let root = fork_root(parent_present.then_some(&parent[..]), &[]);
        write_child(&root, migrated_fork_meta(), &probe_c);
        let ingested = ingest_every_way(&root);
        assert_eq!(own_usage(&ingested, CHILD).0, 1, "the step after the settings counts");
        assert_eq!(own_usage(&ingested, CHILD).1, Some(6));
        assert_excluded_as_gap(&ingested, CHILD);
        assert_eq!(undecided_steps(&ingested), if parent_present { 1 } else { 2 });
    }
}

#[test]
fn a_migrated_forks_first_own_step_is_checked_without_a_copied_counter() {
    // Review A6 on PR #33. The fork names itself before any counter, so no copied total is
    // inherited, and its first counter reports a total its parent reported (so it is not a
    // seed) that its own 20 does not explain. No Codex writer is known for this shape; the
    // check, as for a declared boundary, keeps that total from counting as the fork's own.
    let root = fork_root(Some(&parent_with_two_turns()), &[]);
    write_child(
        &root,
        migrated_fork_meta(),
        &[settings(1, CHILD), turn(2, Some("f-turn-1")), counter(3, 140, 20, 18, 2)],
    );
    let ingested = ingest_every_way(&root);
    assert_eq!(own_usage(&ingested, CHILD).0, 0);
    assert_eq!(counted_totals(&ingested).iter().sum::<u64>(), 160);
    assert_excluded_as_gap(&ingested, CHILD);
    assert_eq!(undecided_steps(&ingested), 1);
}

#[test]
fn a_memory_consolidation_source_is_not_a_subagent_mark() {
    // Codex's migration gives a memory consolidation rollout no subagent boundary
    // (rollout_migration.rs:458-473 at rust-v0.162.1), so a paginated one with a parent
    // link may hold an unmarked copy like a user fork: its turns decide it, and without its
    // parent it is a gap.
    let meta = json!({"parent_thread_id": PARENT, "history_mode": "paginated",
                      "source": {"subagent": "memory_consolidation"}});
    let own = [
        turn(1, Some("c-turn-1")),
        counter(2, 18, 2, 18, 2),
        turn(3, Some("c-turn-2")),
        counter(4, 23, 3, 5, 1),
    ];
    for parent_present in [false, true] {
        let root = fork_root(parent_present.then_some(&parent_with_two_turns()[..]), &[]);
        write_child(&root, meta.clone(), &own);
        let ingested = ingest_every_way(&root);
        if parent_present {
            assert_eq!(own_usage(&ingested, CHILD), (2, Some(26), Completeness::Complete));
            assert_fork_parent_and_complete(&ingested);
        } else {
            assert_eq!(own_usage(&ingested, CHILD).0, 0);
            assert_excluded_as_gap(&ingested, CHILD);
            assert_eq!(undecided_steps(&ingested), 2);
        }
    }
}
