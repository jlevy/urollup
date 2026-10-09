//! Codex rollouts that hold cumulative `token_count` turns before their first
//! `token_usage_record`: a session that a release before 0.153 started and a later
//! release resumed (`uro-h2sf`). Each response must count once, from whichever record
//! reports it.

use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;

use serde_json::{Value, json};
use urollup_core::accounting::totals::{Completeness, ledger_totals, selection_totals};
use urollup_core::adapters::{Ingested, codex_rollout};
use urollup_core::ledger::diagnostics::DiagnosticCode;
use urollup_core::ledger::tokens::TokenMeasures;
use urollup_core::selection::{Agent, agent_thread_identity};
use urollup_core::sources::roots::discover;

const SESSION: &str = "019f0000-0000-7000-8000-00ee00000001";
const PARENT: &str = "019f0000-0000-7000-8000-00ee00000002";
const CHILD: &str = "019f0000-0000-7000-8000-00ee00000003";

/// Usage as Codex writes it: input (cached input included), cached input, output and
/// reasoning output.
type Usage = [u64; 4];

fn usage([input, cached, output, reasoning]: Usage) -> Value {
    json!({
        "input_tokens": input, "cached_input_tokens": cached, "cache_write_input_tokens": 0,
        "output_tokens": output, "reasoning_output_tokens": reasoning,
        "total_tokens": input + output
    })
}

fn meta(thread: &str, parent: Option<&str>) -> String {
    let mut payload = json!({"id": thread, "cwd": "/work/project", "cli_version": "0.150.0"});
    if let Some(parent) = parent {
        payload["parent_thread_id"] = json!(parent);
    }
    json!({"type": "session_meta", "payload": payload}).to_string()
}

fn turn(turn_id: &str) -> String {
    json!({"type": "turn_context", "payload": {"turn_id": turn_id, "model": "gpt-test"}})
        .to_string()
}

fn counter(total: Usage, last: Usage) -> String {
    json!({"type": "event_msg", "payload": {"type": "token_count", "info": {
        "total_token_usage": usage(total), "last_token_usage": usage(last)
    }}})
    .to_string()
}

fn direct(thread: &str, turn_id: Option<&str>, response: &str, own: Usage) -> String {
    let mut payload = json!({"thread_id": thread, "response_id": response, "usage": usage(own)});
    if let Some(turn_id) = turn_id {
        payload["turn_id"] = json!(turn_id);
    }
    json!({"type": "token_usage_record", "payload": payload}).to_string()
}

fn write_rollout(home: &Path, thread: &str, minute: u8, lines: &[String]) {
    let day = home.join("sessions/2026/10/01");
    fs::create_dir_all(&day).expect("rollout directory");
    fs::write(
        day.join(format!("rollout-2026-10-01T00-{minute:02}-00-{thread}.jsonl")),
        lines.join("\n") + "\n",
    )
    .expect("synthetic rollout");
}

fn ingest_session(lines: &[String]) -> Ingested {
    let home = tempfile::tempdir().expect("temporary Codex home");
    write_rollout(home.path(), SESSION, 0, lines);
    codex_rollout::ingest_root(home.path()).expect("the rollout ingests")
}

/// Counted requests and normalized tokens of the whole ledger.
fn counted(ingested: &Ingested) -> (u64, TokenMeasures) {
    let totals = ledger_totals(&ingested.ledger).expect("totals");
    assert_eq!(totals.completeness, Completeness::Complete, "every usage record is accounted");
    (totals.total.requests, totals.total.tokens)
}

/// Counted requests and the token sum of one thread.
fn thread_total(ingested: &Ingested, native: &str) -> (u64, Option<u64>) {
    let thread = agent_thread_identity(Agent::Codex, native).expect("thread identity").id;
    let totals =
        selection_totals(&ingested.ledger, &BTreeSet::from([thread])).expect("selection totals");
    (totals.counted.requests, totals.counted.tokens.total().expect("valid sum"))
}

fn tokens(uncached_input: u64, cache_read: u64, output: u64, reasoning: u64) -> TokenMeasures {
    TokenMeasures {
        uncached_input: Some(uncached_input),
        cache_read: Some(cache_read),
        output: Some(output),
        reasoning: Some(reasoning),
        ..TokenMeasures::default()
    }
}

fn assert_tokens(actual: &TokenMeasures, expected: &TokenMeasures) {
    assert_eq!(
        (actual.uncached_input, actual.cache_read, actual.output, actual.reasoning),
        (expected.uncached_input, expected.cache_read, expected.output, expected.reasoning)
    );
}

#[test]
fn counter_turns_before_the_first_usage_record_count_beside_the_usage_records() {
    let ingested = ingest_session(&[
        meta(SESSION, None),
        // Started by a release that writes cumulative token_count events only.
        turn("turn-1"),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        counter([2_500, 500, 250, 50], [1_500, 500, 150, 50]),
        turn("turn-2"),
        counter([4_500, 1_500, 400, 50], [2_000, 1_000, 150, 0]),
        // Resumed by a release that writes a token_usage_record for every response.
        turn("turn-3"),
        direct(SESSION, Some("turn-3"), "resp-3", [3_000, 2_000, 200, 20]),
        counter([7_500, 3_500, 600, 70], [3_000, 2_000, 200, 20]),
        turn("turn-4"),
        counter([8_700, 4_000, 700, 70], [1_200, 500, 100, 0]),
        direct(SESSION, Some("turn-4"), "resp-4", [1_200, 500, 100, 0]),
    ]);

    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 5, "three counter responses and two usage records");
    // The final cumulative total: every response once.
    assert_tokens(&total, &tokens(8_700 - 4_000, 4_000, 700, 70));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

#[test]
fn a_counter_written_just_before_the_usage_record_of_its_response_counts_once() {
    // The resumed turn's token_count precedes the token_usage_record of the same response.
    let ingested = ingest_session(&[
        meta(SESSION, None),
        turn("turn-1"),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        turn("turn-2"),
        counter([3_000, 800, 300, 0], [2_000, 800, 200, 0]),
        direct(SESSION, Some("turn-2"), "resp-2", [2_000, 800, 200, 0]),
    ]);

    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 2);
    assert_tokens(&total, &tokens(1_000 + 1_200, 800, 300, 0));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

#[test]
fn without_turn_context_a_counter_matching_the_next_usage_record_is_that_response() {
    // No turn_context names a turn, so the counter whose last usage equals the next usage
    // record's usage reports that record's response; the earlier one does not.
    let ingested = ingest_session(&[
        meta(SESSION, None),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        counter([3_000, 800, 300, 0], [2_000, 800, 200, 0]),
        direct(SESSION, None, "resp-2", [2_000, 800, 200, 0]),
    ]);

    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 2);
    assert_tokens(&total, &tokens(1_000 + 1_200, 800, 300, 0));
}

#[test]
fn a_counted_turn_after_a_twin_counter_counts_only_its_own_usage() {
    // Not a shape Codex is known to write: a counter-only turn whose counter follows the
    // counter of a response that a later usage record reports. Its delta must start from
    // that counter's total, or the twin's usage would count twice.
    let ingested = ingest_session(&[
        meta(SESSION, None),
        turn("turn-1"),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        turn("turn-2"),
        counter([3_000, 0, 300, 0], [2_000, 0, 200, 0]),
        turn("turn-3"),
        counter([3_500, 0, 350, 0], [500, 0, 50, 0]),
        direct(SESSION, Some("turn-2"), "resp-2", [2_000, 0, 200, 0]),
    ]);

    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 3);
    assert_tokens(&total, &tokens(3_500, 0, 350, 0));
}

/// A counter-only parent, and its legacy subagent resumed by a release that writes usage
/// records: the subagent's copied prefix, its own counter-only turn, and its resumed
/// turn, whose counter precedes its usage record.
fn legacy_subagent_resumed_after_the_upgrade(home: &Path) {
    let parent_turn = turn("turn-p1");
    let parent_counter = counter([900, 0, 100, 0], [900, 0, 100, 0]);
    write_rollout(
        home,
        PARENT,
        0,
        &[meta(PARENT, None), parent_turn.clone(), parent_counter.clone()],
    );
    write_rollout(
        home,
        CHILD,
        1,
        &[
            meta(CHILD, Some(PARENT)),
            meta(PARENT, None),
            parent_turn,
            parent_counter,
            turn("turn-k1"),
            counter([1_080, 0, 120, 0], [180, 0, 20, 0]),
            turn("turn-k2"),
            counter([1_350, 0, 150, 0], [270, 0, 30, 0]),
            direct(CHILD, Some("turn-k2"), "resp-k2", [270, 0, 30, 0]),
        ],
    );
}

#[test]
fn a_resumed_legacy_subagent_counts_its_own_turns_on_any_worker_count_and_source_order() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    legacy_subagent_resumed_after_the_upgrade(home.path());
    let roots = codex_rollout::rollout_roots(&[home.path().to_owned()]);
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
            .expect("ingest the subagent");

            assert_eq!(thread_total(&ingested, PARENT), (1, Some(1_000)));
            assert_eq!(
                thread_total(&ingested, CHILD),
                (2, Some(200 + 300)),
                "the copied prefix is the parent's; both of the child's own turns count"
            );
            let (requests, _) = counted(&ingested);
            assert_eq!(requests, 3, "workers={workers}, reversed={reversed}");
            assert!(
                ingested
                    .ledger
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagnosticCode::CodexCopiedHistoryInferred),
                "the counter region's copy boundary was inferred from its turns"
            );
            if let Some(expected) = &baseline {
                assert_eq!(&ingested.ledger, expected, "workers={workers}, reversed={reversed}");
            } else {
                baseline = Some(ingested.ledger);
            }
        }
    }
}
