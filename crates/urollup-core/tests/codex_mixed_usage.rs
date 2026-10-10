//! Codex root rollouts that hold cumulative `token_count` usage beside `token_usage_record`
//! lines (`uro-h2sf`): a session that a release before 0.153 started and a later release
//! resumed, or one that an earlier release appended turns to after a later one wrote it.
//! Each response must count once, from whichever record reports it.
//!
//! In a root rollout, a counter is the twin of a usage record when the two are adjacent
//! usage events with exactly the same usage: Codex writes the record first, and urollup
//! also accepts the reverse order. Twins move the running total and add nothing; every
//! other counter that reports usage goes through the counter rules wherever it sits.
//! Forks and subagents keep the accounting they had before (`uro-r8si`).

use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;

use serde_json::{Value, json};
use urollup_core::accounting::totals::{Completeness, ledger_totals, selection_totals};
use urollup_core::adapters::{Ingested, codex_rollout};
use urollup_core::ledger::diagnostics::DiagnosticCode;
use urollup_core::ledger::entities::Counting;
use urollup_core::ledger::tokens::TokenMeasures;
use urollup_core::selection::{Agent, agent_thread_identity};
use urollup_core::sources::roots::discover;

const SESSION: &str = "019f0000-0000-7000-8000-00ee00000001";
const PARENT: &str = "019f0000-0000-7000-8000-00ee00000002";
const CHILD: &str = "019f0000-0000-7000-8000-00ee00000003";
const GRANDCHILD: &str = "019f0000-0000-7000-8000-00ee00000004";
const OTHER: &str = "019f0000-0000-7000-8000-00ee00000005";

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

/// A child's own `session_meta` at ordinal 0 with extra payload fields.
fn child_meta(thread: &str, fields: Value) -> String {
    let mut payload = json!({"id": thread, "cwd": "/work/project", "cli_version": "0.154.0"});
    if let (Some(payload), Value::Object(fields)) = (payload.as_object_mut(), fields) {
        payload.extend(fields);
    }
    json!({"ordinal": 0, "type": "session_meta", "payload": payload}).to_string()
}

fn turn(turn_id: &str) -> String {
    json!({"type": "turn_context", "payload": {"turn_id": turn_id, "model": "gpt-test"}})
        .to_string()
}

/// A `turn_context` without a turn ID, as some older releases write it.
fn turn_without_id() -> String {
    json!({"type": "turn_context", "payload": {"model": "gpt-test"}}).to_string()
}

fn counter(total: Usage, last: Usage) -> String {
    json!({"type": "event_msg", "payload": {"type": "token_count", "info": {
        "total_token_usage": usage(total), "last_token_usage": usage(last)
    }}})
    .to_string()
}

/// A `turn_context` that names its model.
fn turn_model(turn_id: &str, model: &str) -> String {
    json!({"type": "turn_context", "payload": {"turn_id": turn_id, "model": model}}).to_string()
}

/// The line with a record `timestamp`.
fn stamped(timestamp: &str, line: &str) -> String {
    let mut value: Value = serde_json::from_str(line).expect("synthetic line");
    value["timestamp"] = json!(timestamp);
    value.to_string()
}

/// A compaction estimate: the running total unchanged, and a `last_token_usage` with zero
/// input and output and nonzero `total_tokens`.
fn estimate(total: Usage) -> String {
    json!({"type": "event_msg", "payload": {"type": "token_count", "info": {
        "total_token_usage": usage(total),
        "last_token_usage": {
            "input_tokens": 0, "cached_input_tokens": 0, "cache_write_input_tokens": 0,
            "output_tokens": 0, "reasoning_output_tokens": 0, "total_tokens": 3_200
        }
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

fn settings(thread: &str) -> String {
    json!({"type": "event_msg", "payload": {"type": "thread_settings_applied", "thread_id": thread}})
        .to_string()
}

/// A line the adapter skips, such as a message.
fn skipped() -> String {
    json!({"type": "response_item", "payload": {"type": "message", "role": "user"}}).to_string()
}

/// The line with a record `ordinal`.
fn at(ordinal: u64, line: &str) -> String {
    let mut value: Value = serde_json::from_str(line).expect("synthetic line");
    value["ordinal"] = json!(ordinal);
    value.to_string()
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

/// Ingests `home` on 1 and 8 workers in forward and reversed discovery order, asserts
/// that every run gives the same ledger, and returns it.
fn ingest_on_any_worker_count_and_order(home: &Path) -> Ingested {
    let roots = codex_rollout::rollout_roots(&[home.to_owned()]);
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
            .expect("the rollouts ingest");
            if let Some(expected) = &baseline {
                assert_eq!(
                    ingested.ledger, expected.ledger,
                    "workers={workers}, reversed={reversed}"
                );
            } else {
                baseline = Some(ingested);
            }
        }
    }
    baseline.expect("one run")
}

/// Counted requests and normalized tokens of the whole ledger.
fn counted(ingested: &Ingested) -> (u64, TokenMeasures) {
    let totals = ledger_totals(&ingested.ledger).expect("totals");
    assert_eq!(totals.completeness, Completeness::Complete, "every usage record is accounted");
    (totals.total.requests, totals.total.tokens)
}

/// Counted requests, their token sum and the completeness of the whole ledger.
fn summary(ingested: &Ingested) -> (u64, Option<u64>, Completeness) {
    let totals = ledger_totals(&ingested.ledger).expect("totals");
    (totals.total.requests, totals.total.tokens.total().expect("valid sum"), totals.completeness)
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

/// The ledger's diagnostic codes with their occurrences.
fn diagnostics(ingested: &Ingested) -> Vec<(DiagnosticCode, u64)> {
    ingested
        .ledger
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.occurrences))
        .collect()
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
fn a_counted_turn_after_a_twin_counts_only_its_own_usage() {
    // A counter-only turn after a twin continues the twin's total, so its delta excludes
    // the response the twin's record reports.
    let ingested = ingest_session(&[
        meta(SESSION, None),
        turn("turn-1"),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        turn("turn-2"),
        counter([3_000, 0, 300, 0], [2_000, 0, 200, 0]),
        direct(SESSION, Some("turn-2"), "resp-2", [2_000, 0, 200, 0]),
        turn("turn-3"),
        counter([3_500, 0, 350, 0], [500, 0, 50, 0]),
    ]);

    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 3);
    assert_tokens(&total, &tokens(3_500, 0, 350, 0));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

/// Review A1 and B1 (P2): an older release resumed a rollout that a release writing
/// usage records started, and appended a counter-only turn after its first usage record.
#[test]
fn a_counter_only_turn_after_the_first_usage_record_counts() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    write_rollout(
        home.path(),
        SESSION,
        0,
        &[
            meta(SESSION, None),
            turn("t1"),
            direct(SESSION, Some("t1"), "r1", [1_000, 0, 100, 0]),
            counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
            turn("t2"),
            counter([3_000, 0, 300, 0], [2_000, 0, 200, 0]),
            turn("t3"),
            direct(SESSION, Some("t3"), "r3", [500, 0, 50, 0]),
            counter([3_500, 0, 350, 0], [500, 0, 50, 0]),
        ],
    );
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 3, "two usage records and the counter-only turn");
    assert_tokens(&total, &tokens(3_500, 0, 350, 0));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

#[test]
fn counter_only_turns_between_usage_record_turns_count_once_on_any_worker_count_and_order() {
    // Two root sessions with counter-only turns before, between and after usage-record
    // turns: one writes each record before its counter, the other after it. Neither waits
    // for the other rollouts, and the ledger is the same on any worker count and order.
    let home = tempfile::tempdir().expect("temporary Codex home");
    for (minute, thread, record_first) in [(0, SESSION, true), (1, OTHER, false)] {
        let recorded = |id: &str, response: &str, total: Usage, own: Usage| {
            let (record, twin) = (
                direct(thread, Some(id), &format!("{thread}-{response}"), own),
                counter(total, own),
            );
            let pair = if record_first { [record, twin] } else { [twin, record] };
            [vec![turn(id)], pair.to_vec()].concat()
        };
        let lines = [
            vec![meta(thread, None), turn("a1")],
            vec![counter([1_000, 0, 100, 0], [1_000, 0, 100, 0])],
            recorded("a2", "r2", [3_000, 500, 300, 0], [2_000, 500, 200, 0]),
            vec![turn("a3"), counter([4_500, 500, 400, 0], [1_500, 0, 100, 0])],
            recorded("a4", "r4", [5_300, 1_300, 440, 0], [800, 800, 40, 0]),
            vec![turn("a5"), counter([6_000, 1_300, 500, 0], [700, 0, 60, 0])],
        ]
        .concat();
        write_rollout(home.path(), thread, minute, &lines);
    }
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    for thread in [SESSION, OTHER] {
        // 1,100 + 2,200 + 1,600 + 840 + 760: the final running total.
        assert_eq!(thread_total(&ingested, thread), (5, Some(6_500)), "{thread}");
    }
    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 10);
    assert_tokens(&total, &tokens(2 * (6_000 - 1_300), 2 * 1_300, 2 * 500, 0));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

/// Review B2 (P6), with its control P6b: a resumed legacy session whose first recorded
/// response is a compaction written before the new turn's `turn_context`.
#[test]
fn a_compaction_counter_before_the_new_turn_context_is_its_records_twin_in_either_order() {
    for compaction_counter_first in [true, false] {
        let mut compaction = [
            counter([3_400, 0, 340, 0], [400, 0, 40, 0]),
            direct(SESSION, Some("t3"), "r3-compact", [400, 0, 40, 0]),
        ];
        if !compaction_counter_first {
            compaction.swap(0, 1);
        }
        let home = tempfile::tempdir().expect("temporary Codex home");
        let lines = [
            vec![
                meta(SESSION, None),
                turn("t1"),
                counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
                turn("t2"),
                counter([3_000, 0, 300, 0], [2_000, 0, 200, 0]),
            ],
            compaction.to_vec(),
            vec![
                turn("t3"),
                direct(SESSION, Some("t3"), "r3", [600, 0, 60, 0]),
                counter([4_000, 0, 400, 0], [600, 0, 60, 0]),
            ],
        ]
        .concat();
        write_rollout(home.path(), SESSION, 0, &lines);
        let ingested = ingest_on_any_worker_count_and_order(home.path());

        assert_eq!(
            summary(&ingested),
            (4, Some(4_400), Completeness::Complete),
            "compaction_counter_first={compaction_counter_first}"
        );
    }
}

/// Review B3 (P5): without turn IDs, an old counter-only response whose usage equals a
/// later usage record's usage is not that record's twin, since another usage event lies
/// between them.
#[test]
fn an_old_counter_with_a_later_records_usage_is_not_its_twin() {
    let ingested = ingest_session(&[
        meta(SESSION, None),
        counter([500, 0, 50, 0], [500, 0, 50, 0]),
        counter([2_500, 0, 250, 0], [2_000, 0, 200, 0]),
        direct(SESSION, None, "r3", [500, 0, 50, 0]),
        counter([3_000, 0, 300, 0], [500, 0, 50, 0]),
    ]);

    assert_eq!(summary(&ingested), (3, Some(3_300), Completeness::Complete));
}

/// Review B's P1, P3, P4 and P4b: each response counts once whatever the `turn_context`
/// records say, since twins are decided by adjacency and usage, never by turn.
#[test]
fn each_response_counts_once_with_or_without_turn_ids() {
    let p1 = [
        meta(SESSION, None),
        turn("t1"),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        turn("t2"),
        counter([3_000, 0, 300, 0], [2_000, 0, 200, 0]),
        turn("t3"),
        counter([3_500, 0, 350, 0], [500, 0, 50, 0]),
        direct(SESSION, Some("t3"), "r3", [500, 0, 50, 0]),
        counter([4_200, 0, 420, 0], [700, 0, 70, 0]),
        direct(SESSION, Some("t3"), "r3b", [700, 0, 70, 0]),
        turn("t4"),
        counter([5_000, 0, 500, 0], [800, 0, 80, 0]),
        direct(SESSION, Some("t4"), "r4", [800, 0, 80, 0]),
    ];
    let p3 = [
        meta(SESSION, None),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        counter([3_000, 0, 300, 0], [2_000, 0, 200, 0]),
        counter([3_500, 0, 350, 0], [500, 0, 50, 0]),
        direct(SESSION, None, "r3", [500, 0, 50, 0]),
        counter([4_200, 0, 420, 0], [700, 0, 70, 0]),
        direct(SESSION, None, "r4", [700, 0, 70, 0]),
    ];
    let p4 = [
        meta(SESSION, None),
        turn_without_id(),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        turn_without_id(),
        counter([3_000, 0, 300, 0], [2_000, 0, 200, 0]),
        turn("t3"),
        counter([3_500, 0, 350, 0], [500, 0, 50, 0]),
        direct(SESSION, Some("t3"), "r3", [500, 0, 50, 0]),
    ];
    let p4b = [
        meta(SESSION, None),
        turn("t1"),
        counter([1_000, 0, 100, 0], [1_000, 0, 100, 0]),
        turn_without_id(),
        counter([1_500, 0, 150, 0], [500, 0, 50, 0]),
        direct(SESSION, Some("t3"), "r3", [500, 0, 50, 0]),
    ];
    for (name, lines, expected) in [
        ("P1", &p1[..], (5, Some(5_500))),
        ("P3", &p3[..], (4, Some(4_620))),
        ("P4", &p4[..], (3, Some(3_850))),
        ("P4b", &p4b[..], (2, Some(1_650))),
    ] {
        let ingested = ingest_session(lines);
        assert_eq!(summary(&ingested), (expected.0, expected.1, Completeness::Complete), "{name}");
    }
}

#[test]
fn a_counter_with_no_new_usage_follows_the_nearest_usage_event() {
    // A compaction estimate after a counted counter is an estimate diagnostic, as in a
    // rollout without usage records; one after a twin, and a rate-limit refresh repeating
    // the twin's total, add nothing at all, as before usage records were counted beside
    // counters.
    let resumed_total = [190_000, 141_000, 4_400, 1_700];
    let ingested = ingest_session(&[
        meta(SESSION, None),
        turn("t1"),
        counter([120_000, 100_000, 2_000, 800], [120_000, 100_000, 2_000, 800]),
        estimate([120_000, 100_000, 2_000, 800]),
        turn("t2"),
        counter([150_000, 103_000, 3_500, 1_400], [30_000, 3_000, 1_500, 600]),
        turn("t3"),
        direct(SESSION, Some("t3"), "r3", [40_000, 38_000, 900, 300]),
        counter(resumed_total, [40_000, 38_000, 900, 300]),
        estimate(resumed_total),
        counter(resumed_total, [40_000, 38_000, 900, 300]),
    ]);

    let (requests, total) = counted(&ingested);
    assert_eq!(requests, 3);
    // 122,000 + 31,500 + 40,900.
    assert_tokens(&total, &tokens(190_000 - 141_000, 141_000, 4_400, 1_700));
    assert_eq!(diagnostics(&ingested), [(DiagnosticCode::CodexEstimateCompaction, 1)]);
}

/// Each counted request's model, tokens and UTC day, sorted.
fn attribution(ingested: &Ingested) -> Vec<(String, Option<u64>, String)> {
    let mut rows: Vec<_> = ingested
        .ledger
        .requests
        .values()
        .filter(|request| request.counting == Counting::Counted)
        .map(|request| {
            let tokens = request.usage.as_ref().and_then(|usage| {
                TokenMeasures::from(usage.revision.usage).total().expect("valid sum")
            });
            let model =
                request.model.as_ref().map_or_else(String::new, |model| model.name.to_string());
            let day = request.first_seen.map_or_else(String::new, |seen| {
                jiff::Timestamp::from(seen).to_string().chars().take(10).collect()
            });
            (model, tokens, day)
        })
        .collect();
    rows.sort();
    rows
}

/// Review D1 (d6): a legacy counter-only response, then a resumed turn whose first
/// response has exactly the same usage. No pair spans the resumed turn's `turn_context`,
/// so the legacy response keeps its own day and model, and still counts when the resumed
/// record's own `token_count` was never written.
#[test]
fn a_legacy_response_with_the_next_resumed_records_usage_keeps_its_day_and_model() {
    let same = [1_000, 0, 100, 0];
    let legacy = [
        meta(SESSION, None),
        stamped("2026-10-01T10:00:00Z", &turn_model("t1", "legacy-model")),
        stamped("2026-10-01T10:00:05Z", &counter(same, same)),
        stamped("2026-10-02T10:00:00Z", &turn_model("t2", "modern-model")),
        stamped("2026-10-02T10:00:05Z", &direct(SESSION, Some("t2"), "r2", same)),
    ];
    let twin = stamped("2026-10-02T10:00:06Z", &counter([2_000, 0, 200, 0], same));
    for (name, lines) in [
        ("with the record's token_count", [legacy.as_slice(), &[twin]].concat()),
        ("without it", legacy.to_vec()),
    ] {
        let ingested = ingest_session(&lines);
        assert_eq!(summary(&ingested), (2, Some(2_200), Completeness::Complete), "{name}");
        assert_eq!(
            attribution(&ingested),
            [
                ("legacy-model".to_owned(), Some(1_100), "2026-10-01".to_owned()),
                ("modern-model".to_owned(), Some(1_100), "2026-10-02".to_owned()),
            ],
            "{name}"
        );
    }
}

/// Review C5 (c3): consecutive responses with exactly equal usage. Each counter takes the
/// untaken record next to it, the one before it first, so neither order counts a response
/// twice, nor does a legacy response with the same usage as the next record.
#[test]
fn consecutive_responses_with_equal_usage_count_once_in_either_order() {
    let same = [1_000, 0, 100, 0];
    let reverse = [
        meta(SESSION, None),
        turn("t1"),
        counter([1_000, 0, 100, 0], same),
        direct(SESSION, Some("t1"), "r1", same),
        turn("t2"),
        counter([2_000, 0, 200, 0], same),
        direct(SESSION, Some("t2"), "r2", same),
    ];
    let codex = [
        meta(SESSION, None),
        turn("t1"),
        direct(SESSION, Some("t1"), "r1", same),
        counter([1_000, 0, 100, 0], same),
        turn("t2"),
        direct(SESSION, Some("t2"), "r2", same),
        counter([2_000, 0, 200, 0], same),
    ];
    let legacy_then_reverse = [
        meta(SESSION, None),
        turn("t0"),
        counter([1_000, 0, 100, 0], same),
        turn("t1"),
        counter([2_000, 0, 200, 0], same),
        direct(SESSION, Some("t1"), "r1", same),
    ];
    for (name, lines) in
        [("reverse", &reverse[..]), ("Codex", &codex[..]), ("legacy", &legacy_then_reverse[..])]
    {
        let ingested = ingest_session(lines);
        assert_eq!(summary(&ingested), (2, Some(2_200), Completeness::Complete), "{name}");
    }
}

#[test]
fn a_rollout_whose_counters_are_all_twins_never_reads_their_totals() {
    // As before counters were counted beside usage records, a twin's running total is
    // not read when no counter of the rollout is counted, so a total that could not be
    // normalized (cached input above input) still ingests.
    let ingested = ingest_session(&[
        meta(SESSION, None),
        turn("t1"),
        direct(SESSION, Some("t1"), "r1", [1_000, 0, 100, 0]),
        counter([1_000, 2_000, 100, 0], [1_000, 0, 100, 0]),
    ]);

    assert_eq!(summary(&ingested), (1, Some(1_100), Completeness::Complete));
}

// Rollouts that are not roots: forks and subagents keep the accounting they had before
// counters were counted beside usage records. Each test pins that accounting; the gaps
// it shows are tracked as uro-r8si.

/// A legacy parent with two counter-only turns, and its 0.154 fork, whose copied prefix
/// keeps the parent's counters before the fork's first usage record.
fn legacy_fork(home: &Path, parent_written: bool) {
    let parent_lines = [
        turn("turn-p1"),
        counter([900, 0, 100, 0], [900, 0, 100, 0]),
        turn("turn-p2"),
        counter([1_800, 0, 200, 0], [900, 0, 100, 0]),
    ];
    if parent_written {
        write_rollout(home, PARENT, 0, &[[meta(PARENT, None)].as_slice(), &parent_lines].concat());
    }
    let child = [
        vec![fork_meta(CHILD, PARENT), meta(PARENT, None)],
        parent_lines.to_vec(),
        vec![
            // The fork's first response in the reverse order, its second in Codex's order.
            turn("turn-k1"),
            counter([2_070, 0, 230, 0], [270, 0, 30, 0]),
            direct(CHILD, Some("turn-k1"), "resp-k1", [270, 0, 30, 0]),
            turn("turn-k2"),
            direct(CHILD, Some("turn-k2"), "resp-k2", [180, 0, 20, 0]),
            counter([2_250, 0, 250, 0], [180, 0, 20, 0]),
        ],
    ]
    .concat();
    write_rollout(home, CHILD, 1, &child);
}

fn fork_meta(thread: &str, from: &str) -> String {
    json!({"type": "session_meta", "payload": {"id": thread, "forked_from_id": from}}).to_string()
}

/// Review B's P7 and P7b: a 0.154 fork of a legacy parent, with and without the parent.
/// The fork's usage records account for its usage, and its copied counters stay copies.
#[test]
fn a_fork_of_a_legacy_parent_counts_its_usage_records_once() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    legacy_fork(home.path(), true);
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(thread_total(&ingested, PARENT), (2, Some(2_000)));
    assert_eq!(thread_total(&ingested, CHILD), (2, Some(300 + 200)));
    assert_eq!(summary(&ingested), (4, Some(2_500), Completeness::Complete));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);

    let home = tempfile::tempdir().expect("temporary Codex home");
    legacy_fork(home.path(), false);
    let ingested = ingest_on_any_worker_count_and_order(home.path());
    assert_eq!(thread_total(&ingested, CHILD), (2, Some(500)), "the parent rollout is missing");
}

#[test]
fn a_forks_copied_counter_stays_a_copy_beside_the_forks_first_record() {
    // The fork's last copied counter reports exactly the usage of the fork's first usage
    // record: the copy stays a copy and the fork's own counter adds nothing.
    let home = tempfile::tempdir().expect("temporary Codex home");
    let parent_lines = [turn("turn-p1"), counter([900, 0, 100, 0], [900, 0, 100, 0])];
    write_rollout(
        home.path(),
        PARENT,
        0,
        &[[meta(PARENT, None)].as_slice(), &parent_lines].concat(),
    );
    write_rollout(
        home.path(),
        CHILD,
        1,
        &[
            [meta(CHILD, Some(PARENT)), meta(PARENT, None)].as_slice(),
            &parent_lines,
            &[
                settings(CHILD),
                turn("turn-k1"),
                direct(CHILD, Some("turn-k1"), "resp-k1", [900, 0, 100, 0]),
                counter([1_800, 0, 200, 0], [900, 0, 100, 0]),
            ],
        ]
        .concat(),
    );
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(thread_total(&ingested, PARENT), (1, Some(1_000)));
    assert_eq!(thread_total(&ingested, CHILD), (1, Some(1_000)));
    assert_eq!(summary(&ingested), (2, Some(2_000), Completeness::Complete));
}

/// The child line numbers, from 1, of each counted request's copies.
fn copy_lines(ingested: &Ingested, child: &[String]) -> Vec<(Option<u64>, Vec<usize>)> {
    let mut offsets = Vec::new();
    let mut offset = 0_u64;
    for line in child {
        offsets.push(offset);
        offset += u64::try_from(line.len()).expect("line length") + 1;
    }
    let mut lines: Vec<_> = ingested
        .ledger
        .requests
        .values()
        .filter(|request| request.counting == Counting::Counted)
        .map(|request| {
            let tokens = request.usage.as_ref().map(|usage| {
                TokenMeasures::from(usage.revision.usage)
                    .total()
                    .expect("valid sum")
                    .expect("known tokens")
            });
            let mut copies: Vec<usize> = request
                .copies()
                .iter()
                .filter_map(|copy| offsets.iter().position(|offset| *offset == copy.offset))
                .map(|index| index + 1)
                .collect();
            copies.sort_unstable();
            (tokens, copies)
        })
        .collect();
    lines.sort();
    lines
}

/// Review B's P10: a declared-boundary child whose copied prefix holds the parent's usage
/// records and counters. A copied counter joins the copied thread's latest response, which
/// in Codex's order is its own; totals are the same in either order.
#[test]
fn a_childs_copied_counter_joins_the_copied_threads_latest_response() {
    for counter_first in [true, false] {
        let home = tempfile::tempdir().expect("temporary Codex home");
        let responses = |base: u64| {
            let c1 = at(base + 1, &counter([900, 0, 100, 0], [900, 0, 100, 0]));
            let u1 = at(base + 2, &direct(PARENT, Some("turn-p1"), "resp-p1", [900, 0, 100, 0]));
            let c2 = at(base + 3, &counter([1_350, 0, 150, 0], [450, 0, 50, 0]));
            let u2 = at(base + 4, &direct(PARENT, Some("turn-p1"), "resp-p2", [450, 0, 50, 0]));
            let pairs = if counter_first { [c1, u1, c2, u2] } else { [u1, c1, u2, c2] };
            [vec![at(base, &turn("turn-p1"))], pairs.to_vec()].concat()
        };
        let parent = [vec![child_meta(PARENT, json!({}))], responses(1)].concat();
        write_rollout(home.path(), PARENT, 0, &parent);
        let child = [
            vec![child_meta(
                CHILD,
                json!({"parent_thread_id": PARENT, "forked_from_id": PARENT,
                       "history_mode": "paginated", "subagent_history_start_ordinal": 6}),
            )],
            responses(1),
            vec![
                at(6, &turn("turn-k1")),
                at(7, &direct(CHILD, Some("turn-k1"), "resp-k1", [180, 0, 20, 0])),
                at(8, &counter([1_530, 0, 170, 0], [180, 0, 20, 0])),
            ],
        ]
        .concat();
        write_rollout(home.path(), CHILD, 1, &child);
        let ingested = ingest_on_any_worker_count_and_order(home.path());

        assert_eq!(summary(&ingested), (3, Some(1_700), Completeness::Complete));
        let expected = if counter_first {
            // The reverse order, which Codex does not write: a copied counter joins the
            // previous response, and the first one, with none before it, stays unkeyed.
            vec![(Some(200), vec![]), (Some(500), vec![6]), (Some(1_000), vec![4, 5])]
        } else {
            vec![(Some(200), vec![]), (Some(500), vec![5, 6]), (Some(1_000), vec![3, 4])]
        };
        assert_eq!(copy_lines(&ingested, &child), expected, "counter_first={counter_first}");
    }
}

#[test]
fn a_copied_rate_limit_refresh_joins_the_response_its_twin_reports() {
    // A 0.154 fork whose copied prefix keeps the parent's usage record, its twin and a
    // refresh repeating the twin's total: the copies join the parent's response.
    let home = tempfile::tempdir().expect("temporary Codex home");
    let parent_lines = [
        settings(PARENT),
        turn("turn-p1"),
        direct(PARENT, Some("turn-p1"), "resp-p1", [900, 0, 100, 0]),
        counter([900, 0, 100, 0], [900, 0, 100, 0]),
        counter([900, 0, 100, 0], [900, 0, 100, 0]),
    ];
    write_rollout(
        home.path(),
        PARENT,
        0,
        &[[meta(PARENT, None)].as_slice(), &parent_lines].concat(),
    );
    let child = [
        vec![fork_meta(CHILD, PARENT), meta(PARENT, None)],
        parent_lines.to_vec(),
        vec![
            settings(CHILD),
            turn("turn-k1"),
            direct(CHILD, Some("turn-k1"), "resp-k1", [180, 0, 20, 0]),
            counter([1_080, 0, 120, 0], [180, 0, 20, 0]),
        ],
    ]
    .concat();
    write_rollout(home.path(), CHILD, 1, &child);
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(summary(&ingested), (2, Some(1_200), Completeness::Complete));
    assert_eq!(ledger_totals(&ingested.ledger).expect("totals").copy_only.requests, 0);
    assert_eq!(copy_lines(&ingested, &child), [(Some(200), vec![]), (Some(1_000), vec![5, 6, 7])]);
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

/// A declared-boundary fork whose prefix keeps the parent's turn and counter but drops its
/// usage record (review B's P11 and P12).
fn declared_boundary_fork(home: &Path, copied_ordinal: Option<u64>) {
    let parent = [
        child_meta(PARENT, json!({})),
        at(1, &turn("turn-p1")),
        at(2, &direct(PARENT, Some("turn-p1"), "resp-p1", [900, 0, 100, 0])),
        at(3, &counter([900, 0, 100, 0], [900, 0, 100, 0])),
    ];
    write_rollout(home, PARENT, 0, &parent);
    let copied = counter([900, 0, 100, 0], [900, 0, 100, 0]);
    write_rollout(
        home,
        CHILD,
        1,
        &[
            child_meta(
                CHILD,
                json!({"parent_thread_id": PARENT, "forked_from_id": PARENT,
                       "history_mode": "paginated", "subagent_history_start_ordinal": 3}),
            ),
            at(1, &turn("turn-p1")),
            copied_ordinal.map_or(copied.clone(), |ordinal| at(ordinal, &copied)),
            at(3, &turn("turn-k1")),
            at(4, &direct(CHILD, Some("turn-k1"), "resp-k1", [180, 0, 20, 0])),
            at(5, &counter([1_080, 0, 120, 0], [180, 0, 20, 0])),
        ],
    );
}

/// Review B4 (P11 and P12): a declared-boundary fork's copied counter, with or without an
/// ordinal, is a copy of the parent's usage, and coverage stays complete.
#[test]
fn a_declared_boundary_forks_copied_counter_is_a_copy_with_or_without_an_ordinal() {
    for copied_ordinal in [Some(2), None] {
        let home = tempfile::tempdir().expect("temporary Codex home");
        declared_boundary_fork(home.path(), copied_ordinal);
        let ingested = ingest_on_any_worker_count_and_order(home.path());
        assert_eq!(
            summary(&ingested),
            (2, Some(1_200), Completeness::Complete),
            "copied_ordinal={copied_ordinal:?}"
        );
        assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
    }
}

/// Review B's P8: a declared-boundary child with a copied counter, an own counter-only
/// turn, and a resumed turn written in Codex's order. Its usage records account for its
/// usage, so the counter-only turn before them is not counted (uro-r8si).
#[test]
fn a_resumed_declared_boundary_child_counts_its_usage_records() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    write_rollout(
        home.path(),
        PARENT,
        0,
        &[
            child_meta(PARENT, json!({})),
            at(1, &turn("turn-p1")),
            at(2, &counter([900, 0, 100, 0], [900, 0, 100, 0])),
        ],
    );
    write_rollout(
        home.path(),
        CHILD,
        1,
        &[
            child_meta(
                CHILD,
                json!({"parent_thread_id": PARENT, "forked_from_id": PARENT,
                       "history_mode": "paginated", "subagent_history_start_ordinal": 3}),
            ),
            at(1, &counter([900, 0, 100, 0], [900, 0, 100, 0])),
            at(3, &turn("turn-k1")),
            at(4, &counter([1_080, 0, 120, 0], [180, 0, 20, 0])),
            at(5, &turn("turn-k2")),
            at(6, &direct(CHILD, Some("turn-k2"), "resp-k2", [270, 0, 30, 0])),
            at(7, &counter([1_350, 0, 150, 0], [270, 0, 30, 0])),
        ],
    );
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(thread_total(&ingested, PARENT), (1, Some(1_000)));
    assert_eq!(thread_total(&ingested, CHILD), (1, Some(300)));
    assert_eq!(summary(&ingested).2, Completeness::Complete);
}

/// A counter-only parent, and its legacy subagent resumed by a release that writes usage
/// records: the subagent's copied prefix, its own counter-only turn, and its resumed
/// turn, whose counter precedes its usage record. Skipped lines sit inside the copied
/// prefix and after it.
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
            skipped(),
            parent_turn,
            parent_counter,
            turn("turn-k1"),
            skipped(),
            counter([1_080, 0, 120, 0], [180, 0, 20, 0]),
            turn("turn-k2"),
            counter([1_350, 0, 150, 0], [270, 0, 30, 0]),
            direct(CHILD, Some("turn-k2"), "resp-k2", [270, 0, 30, 0]),
        ],
    );
}

/// A resumed legacy subagent: its usage record counts, and its counter-only turn before
/// that record is not counted (uro-r8si), on any worker count and discovery order.
#[test]
fn a_resumed_legacy_subagent_counts_its_usage_records_on_any_worker_count_and_order() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    legacy_subagent_resumed_after_the_upgrade(home.path());
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(thread_total(&ingested, PARENT), (1, Some(1_000)));
    assert_eq!(thread_total(&ingested, CHILD), (1, Some(300)));
    assert_eq!(summary(&ingested), (2, Some(1_300), Completeness::Complete));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

/// The legacy root `PARENT`, and its legacy fork `CHILD` with one own counter-only
/// response (500 tokens); returns the child's lines.
fn legacy_fork_of_a_legacy_root(home: &Path) -> Vec<String> {
    let root = [meta(PARENT, None), turn("p1"), counter([900, 0, 100, 0], [900, 0, 100, 0])];
    write_rollout(home, PARENT, 0, &root);
    let child = vec![
        fork_meta(CHILD, PARENT),
        meta(PARENT, None),
        turn("p1"),
        counter([900, 0, 100, 0], [900, 0, 100, 0]),
        turn("c1"),
        counter([1_350, 0, 150, 0], [450, 0, 50, 0]),
    ];
    write_rollout(home, CHILD, 1, &child);
    child
}

/// The three threads' counted requests and tokens, and the ledger's.
fn chain_totals(ingested: &Ingested) -> [(u64, Option<u64>); 4] {
    let (requests, tokens, completeness) = summary(ingested);
    assert_eq!(completeness, Completeness::Complete);
    [
        thread_total(ingested, PARENT),
        thread_total(ingested, CHILD),
        thread_total(ingested, GRANDCHILD),
        (requests, tokens),
    ]
}

/// Review C1 (c2 and c2c): a 0.154 fork, and a 0.154 full-history subagent, of a legacy
/// fork or subagent. The nested copied prefix stays a copy, so the intermediate thread's
/// usage counts once.
#[test]
fn a_modern_fork_or_subagent_of_a_legacy_child_counts_the_childs_usage_once() {
    for subagent in [false, true] {
        let home = tempfile::tempdir().expect("temporary Codex home");
        let mut child = legacy_fork_of_a_legacy_root(home.path());
        if subagent {
            child[0] = meta(CHILD, Some(PARENT));
            write_rollout(home.path(), CHILD, 1, &child);
        }
        let header =
            if subagent { meta(GRANDCHILD, Some(CHILD)) } else { fork_meta(GRANDCHILD, CHILD) };
        let grandchild = [
            vec![header],
            child,
            vec![
                settings(GRANDCHILD),
                turn("g1"),
                direct(GRANDCHILD, Some("g1"), "rg1", [180, 0, 20, 0]),
                counter([1_530, 0, 170, 0], [180, 0, 20, 0]),
            ],
        ]
        .concat();
        write_rollout(home.path(), GRANDCHILD, 2, &grandchild);
        let ingested = ingest_on_any_worker_count_and_order(home.path());

        assert_eq!(
            chain_totals(&ingested),
            [(1, Some(1_000)), (1, Some(500)), (1, Some(200)), (3, Some(1_700))],
            "subagent={subagent}"
        );
        // The legacy child's own inferred copy of the root; none for the grandchild.
        assert_eq!(diagnostics(&ingested), [(DiagnosticCode::CodexCopiedHistoryInferred, 3)]);
    }
}

/// Review C1 (c2b), on `main` as here: in an all-legacy chain the grandchild's copy of the
/// legacy child's own turn ends at that turn, which the root never recorded, so the
/// child's 500 tokens count again as the grandchild's (uro-r8si).
#[test]
fn a_legacy_fork_of_a_legacy_fork_counts_the_intermediate_usage_again() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    let child = legacy_fork_of_a_legacy_root(home.path());
    let grandchild = [
        vec![fork_meta(GRANDCHILD, CHILD)],
        child,
        vec![turn("g1"), counter([1_530, 0, 170, 0], [180, 0, 20, 0])],
    ]
    .concat();
    write_rollout(home.path(), GRANDCHILD, 2, &grandchild);
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(
        chain_totals(&ingested),
        [(1, Some(1_000)), (1, Some(500)), (2, Some(700)), (4, Some(2_200))]
    );
}

/// Review C1 (c6): a legacy subagent resumed by 0.154, which then spawns a full-history
/// subagent whose copied prefix keeps the resumed subagent's counters but not its usage
/// record. The resumed subagent counts its usage record only (uro-r8si), and the new
/// subagent's copy stays a copy.
#[test]
fn a_modern_subagent_of_a_resumed_legacy_subagent_counts_each_usage_record_once() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    write_rollout(
        home.path(),
        PARENT,
        0,
        &[meta(PARENT, None), turn("p1"), counter([900, 0, 100, 0], [900, 0, 100, 0])],
    );
    let child = vec![
        meta(CHILD, Some(PARENT)),
        meta(PARENT, None),
        turn("p1"),
        counter([900, 0, 100, 0], [900, 0, 100, 0]),
        turn("c1"),
        counter([1_350, 0, 150, 0], [450, 0, 50, 0]),
        turn("c2"),
        direct(CHILD, Some("c2"), "rc2", [270, 0, 30, 0]),
        counter([1_620, 0, 180, 0], [270, 0, 30, 0]),
    ];
    write_rollout(home.path(), CHILD, 1, &child);
    let grandchild = [
        vec![meta(GRANDCHILD, Some(CHILD))],
        child.iter().filter(|line| !line.contains("token_usage_record")).cloned().collect(),
        vec![
            settings(GRANDCHILD),
            turn("g1"),
            direct(GRANDCHILD, Some("g1"), "rg1", [180, 0, 20, 0]),
            counter([1_800, 0, 200, 0], [180, 0, 20, 0]),
        ],
    ]
    .concat();
    write_rollout(home.path(), GRANDCHILD, 2, &grandchild);
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(
        chain_totals(&ingested),
        [(1, Some(1_000)), (1, Some(300)), (1, Some(200)), (3, Some(1_500))]
    );
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

/// Review C2 (c1c): a resumed legacy full-history subagent of a subagent that copied
/// nothing. Its usage record keeps its own owner; the counter-only turn before it is not
/// counted, since no turn ends a copy of a non-root thread (uro-r8si).
#[test]
fn a_resumed_subagent_of_a_non_root_parent_keeps_its_records_owner() {
    let home = tempfile::tempdir().expect("temporary Codex home");
    write_rollout(
        home.path(),
        PARENT,
        0,
        &[meta(PARENT, None), turn("p1"), counter([900, 0, 100, 0], [900, 0, 100, 0])],
    );
    let child = [meta(CHILD, Some(PARENT)), turn("c1"), counter([450, 0, 50, 0], [450, 0, 50, 0])];
    write_rollout(home.path(), CHILD, 1, &child);
    let grandchild = [
        vec![meta(GRANDCHILD, Some(CHILD))],
        child.to_vec(),
        vec![
            turn("g1"),
            counter([630, 0, 70, 0], [180, 0, 20, 0]),
            turn("g2"),
            direct(GRANDCHILD, Some("g2"), "rg2", [270, 0, 30, 0]),
            counter([900, 0, 100, 0], [270, 0, 30, 0]),
        ],
    ]
    .concat();
    write_rollout(home.path(), GRANDCHILD, 2, &grandchild);
    let ingested = ingest_on_any_worker_count_and_order(home.path());

    assert_eq!(
        chain_totals(&ingested),
        [(1, Some(1_000)), (1, Some(500)), (1, Some(300)), (3, Some(1_800))]
    );
    let thread = agent_thread_identity(Agent::Codex, GRANDCHILD).expect("thread identity").id;
    let selected =
        selection_totals(&ingested.ledger, &BTreeSet::from([thread])).expect("selection totals");
    assert_eq!(selected.completeness, Completeness::Complete, "no request is ambiguous");
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}
