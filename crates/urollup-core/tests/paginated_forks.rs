//! Explicit fork boundaries must work without an embedded parent session header.

use std::fs;
use std::num::NonZeroUsize;

use serde_json::json;
use urollup_core::adapters::codex_rollout;
use urollup_core::ledger::entities::{Counting, Ownership};
use urollup_core::ledger::tokens::TokenMeasures;
use urollup_core::selection::{Agent, agent_thread_identity};
use urollup_core::sources::roots::discover;

const PARENT: &str = "11111111-1111-4111-8111-111111111111";
const CHILD: &str = "22222222-2222-4222-8222-222222222222";

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

fn direct(ordinal: u64, response: &str, input: u64, output: u64) -> String {
    json!({"ordinal": ordinal, "type": "token_usage_record", "payload": {
        "response_id": response, "usage": {
            "input_tokens": input, "cached_input_tokens": 0,
            "output_tokens": output, "reasoning_output_tokens": 0,
            "total_tokens": input + output
        }
    }})
    .to_string()
}

fn compacted_parent() -> String {
    let record: serde_json::Value = serde_json::from_str(&direct(2, "parent-response", 90, 10))
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
        &[direct(1, "parent-response", 90, 10), direct(3, "child-response", 18, 2)],
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

#[test]
fn explicit_boundary_with_unpositioned_usage_is_rejected() {
    for record in [counter(1, 90, 10, 90, 10), direct(1, "parent-response", 90, 10)] {
        let mut record: serde_json::Value =
            serde_json::from_str(&record).expect("synthetic record");
        record.as_object_mut().expect("record object").remove("ordinal");
        let root = fork_root(None, &[record.to_string()]);
        let error = codex_rollout::ingest_root(root.path())
            .expect_err("a declared boundary requires a usage ordinal");
        assert!(error.to_string().contains("missing usage ordinal"), "{error}");
        assert!(error.to_string().contains(CHILD), "the error identifies the affected rollout");
    }
}

#[test]
fn a_child_counter_without_its_inherited_baseline_is_not_reported_as_new_usage() {
    for parent_known in [true, false] {
        let root = fork_root(None, &[counter(3, 108, 12, 18, 2)]);
        if !parent_known {
            let path = root
                .path()
                .join("sessions")
                .join(format!("rollout-2026-01-01T00-00-02-{CHILD}.jsonl"));
            let metadata = json!({"ordinal": 0, "type": "session_meta", "payload": {
                "id": CHILD, "subagent_history_start_ordinal": 3
            }});
            fs::write(path, format!("{metadata}\n{}\n", counter(3, 108, 12, 18, 2)))
                .expect("missing parent and baseline");
        }
        let error = codex_rollout::ingest_root(root.path())
            .expect_err("the first child cumulative total is not its own usage");
        assert!(error.to_string().contains("missing inherited counter baseline"), "{error}");
    }
}

#[test]
fn a_prefix_without_parent_identity_is_excluded_without_losing_child_usage() {
    for records in [
        vec![counter(1, 90, 10, 90, 10), counter(3, 108, 12, 18, 2)],
        vec![direct(1, "parent-response", 90, 10), direct(3, "child-response", 18, 2)],
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
        let child_path =
            root.path().join("sessions").join(format!("rollout-2026-01-01T00-00-02-{CHILD}.jsonl"));
        let metadata = json!({"ordinal": 0, "type": "session_meta", "payload": {
            "id": CHILD, "parent_thread_id": PARENT, "subagent_history_start_ordinal": boundary
        }});
        fs::write(child_path, format!("{metadata}\n{}\n", counter(1, 90, 10, 90, 10)))
            .expect("malformed boundary metadata");
        let error =
            codex_rollout::ingest_root(root.path()).expect_err("invalid boundary must be visible");
        assert!(error.to_string().contains("invalid history-start ordinal"), "{error}");
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
                        vec![direct(1, "parent-response", 90, 10), inherited],
                        vec![
                            direct(1, "parent-response", 90, 10),
                            counter(2, 90, 10, 90, 10),
                            compacted_parent(),
                            direct(3, "child-response", 18, 2),
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
