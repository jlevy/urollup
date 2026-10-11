//! A Codex usage object whose input is below its cache reads plus writes cannot be
//! normalized. Its record or counter step is excluded with a diagnostic and a coverage
//! gap, and the rest of the rollout and every other rollout still report.

use std::fmt::Write;
use std::fs;

use urollup_core::adapters::Ingested;
use urollup_core::adapters::codex_rollout;
use urollup_core::ledger::coverage::UnobservedReason;
use urollup_core::ledger::tokens::TokenMeasures;

const THREAD: &str = "11111111-1111-4111-8111-111111111111";

fn ingest(lines: &[serde_json::Value]) -> Ingested {
    let root = tempfile::tempdir().expect("temporary directory is created");
    let mut text = String::new();
    for line in lines {
        writeln!(text, "{line}").expect("writing synthetic JSON into a String cannot fail");
    }
    fs::write(root.path().join(format!("rollout-{THREAD}.jsonl")), text)
        .expect("rollout is writable");
    codex_rollout::ingest_root(root.path()).expect("the rollout ingests")
}

fn usage(input: u64, cached: u64, write: u64, output: u64) -> serde_json::Value {
    serde_json::json!({
        "input_tokens": input, "cached_input_tokens": cached,
        "cache_write_input_tokens": write, "output_tokens": output,
    })
}

fn record(response: &str, usage: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({"type":"token_usage_record","payload":{"thread_id":THREAD,"turn_id":"turn","response_id":response,"usage":usage}})
}

fn counter(total: &serde_json::Value, last: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":total,"last_token_usage":last}}})
}

fn header() -> [serde_json::Value; 2] {
    [
        serde_json::json!({"type":"session_meta","payload":{"id":THREAD,"cli_version":"0.154.0"}}),
        serde_json::json!({"type":"turn_context","payload":{"turn_id":"turn","model":"synthetic-model"}}),
    ]
}

/// Each counted request's uncached input, cache reads, cache writes and output.
fn counted(ingested: &Ingested) -> Vec<[Option<u64>; 4]> {
    let mut rows: Vec<_> = ingested
        .ledger
        .requests
        .values()
        .filter_map(|request| request.usage.as_ref())
        .map(|usage| {
            let usage = TokenMeasures::from(usage.revision.usage);
            [usage.uncached_input, usage.cache_read, usage.cache_write_unspecified, usage.output]
        })
        .collect();
    rows.sort();
    rows
}

/// The rollout's diagnostics as code, occurrences and detail.
fn diagnostics(ingested: &Ingested) -> Vec<(&'static str, u64, &str)> {
    ingested
        .ledger
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (diagnostic.code.token(), diagnostic.occurrences, diagnostic.detail.as_str())
        })
        .collect()
}

fn assert_one_gap(ingested: &Ingested, samples: usize) {
    let gaps = &ingested.ledger.gaps;
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert_eq!(gaps[0].reason, UnobservedReason::InconsistentUsage);
    assert!(gaps[0].thread.is_some(), "the gap belongs to the rollout's thread");
    assert_eq!(gaps[0].evidence.len(), samples);
}

#[test]
fn records_below_their_cache_writes_or_reads_are_excluded_and_the_rest_counts() {
    let mut lines = header().to_vec();
    lines.extend([
        // Below reads plus writes: 100 < 60 + 50.
        record("writes", &usage(100, 60, 50, 5)),
        // Below reads alone, which main also refused: 10 < 11.
        record("reads", &usage(10, 11, 0, 1)),
        record("valid", &usage(40, 10, 0, 4)),
    ]);
    let ingested = ingest(&lines);
    assert_eq!(counted(&ingested), [[Some(30), Some(10), Some(0), Some(4)]]);
    assert_eq!(
        diagnostics(&ingested),
        [(
            "codex-inclusive-input-underflow",
            2,
            "Codex input_tokens 100 is below its 60 cached and 50 cache-write tokens at \
             rollout-11111111-1111-4111-8111-111111111111.jsonl:3, so that usage is excluded \
             as a coverage gap; 2 records or counter steps of this rollout are excluded",
        )]
    );
    assert_one_gap(&ingested, 2);
}

#[test]
fn an_inconsistent_cumulative_total_never_lets_earlier_usage_count_again() {
    let mut lines = header().to_vec();
    lines.extend([
        counter(&usage(100, 20, 10, 5), &usage(100, 20, 10, 5)),
        // The total cannot be normalized: 150 < 140 + 20.
        counter(&usage(150, 140, 20, 10), &usage(50, 120, 10, 5)),
        // Its delta from the last readable total also covers the inconsistent step, so it
        // does not match its own usage and is excluded too.
        counter(&usage(300, 160, 30, 15), &usage(100, 20, 10, 5)),
        counter(&usage(400, 180, 40, 20), &usage(100, 20, 10, 5)),
    ]);
    let ingested = ingest(&lines);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]; 2]);
    assert_eq!(
        diagnostics(&ingested),
        [(
            "codex-inclusive-input-underflow",
            2,
            "Codex input_tokens 150 is below its 140 cached and 20 cache-write tokens at \
             rollout-11111111-1111-4111-8111-111111111111.jsonl:4, so that usage is excluded \
             as a coverage gap; 2 records or counter steps of this rollout are excluded",
        )]
    );
    assert_one_gap(&ingested, 2);
}

#[test]
fn a_step_whose_own_usage_is_inconsistent_keeps_its_total_as_the_next_baseline() {
    let mut lines = header().to_vec();
    lines.extend([
        counter(&usage(100, 20, 10, 5), &usage(100, 20, 10, 5)),
        // The total is readable; only its last usage is not: 10 < 20.
        counter(&usage(200, 40, 20, 10), &usage(10, 20, 0, 5)),
        counter(&usage(300, 60, 30, 15), &usage(100, 20, 10, 5)),
    ]);
    let ingested = ingest(&lines);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]; 2]);
    let found = diagnostics(&ingested);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), ("codex-inclusive-input-underflow", 1));
    assert_one_gap(&ingested, 1);
}

#[test]
fn an_inconsistent_first_total_leaves_no_baseline_to_count_from_zero() {
    let mut lines = header().to_vec();
    lines.extend([
        // The first total cannot be normalized: 100 < 90 + 20.
        counter(&usage(100, 90, 20, 5), &usage(100, 90, 20, 5)),
        // Counting from zero would charge the inconsistent step's usage again, so the next
        // total is checked against its own usage and excluded.
        counter(&usage(300, 110, 40, 10), &usage(200, 20, 20, 5)),
        counter(&usage(400, 130, 50, 15), &usage(100, 20, 10, 5)),
    ]);
    let ingested = ingest(&lines);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]]);
    let found = diagnostics(&ingested);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), ("codex-inclusive-input-underflow", 2));
    assert_one_gap(&ingested, 2);
}

#[test]
fn a_copy_with_inconsistent_usage_is_only_evidence() {
    let mut lines = header().to_vec();
    lines.extend([
        // A record naming another thread is that thread's copy, which never counts here.
        serde_json::json!({"type":"token_usage_record","payload":{"thread_id":"22222222-2222-4222-8222-222222222222","turn_id":"turn","response_id":"copied","usage":usage(100, 60, 50, 5)}}),
        record("valid", &usage(40, 10, 0, 4)),
    ]);
    let ingested = ingest(&lines);
    assert_eq!(counted(&ingested), [[Some(30), Some(10), Some(0), Some(4)]]);
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
    assert!(ingested.ledger.gaps.is_empty());
    assert_eq!(ingested.ledger.coverage.copies, 1);
}
