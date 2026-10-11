//! A Codex usage object whose input is below its cache reads plus writes cannot be
//! normalized. Where the rollout would count it, its record or counter step is excluded
//! with a diagnostic and a coverage gap, and the rest of the rollout and every other
//! rollout still report. A copy with such usage keeps only its evidence.
//!
//! Every case is ingested with 1 and 8 workers in both discovery orders, which must build
//! the same ledger.

use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroUsize;

use serde_json::{Value, json};
use urollup_core::accounting::totals::{Completeness, PartialReason, selection_totals};
use urollup_core::adapters::{Ingested, codex_rollout};
use urollup_core::ledger::coverage::UnobservedReason;
use urollup_core::ledger::entities::Counting;
use urollup_core::ledger::tokens::TokenMeasures;
use urollup_core::selection::{Agent, agent_thread_identity};
use urollup_core::sources::roots::discover;

const ROOT: &str = "11111111-1111-4111-8111-111111111111";
const PARENT: &str = "22222222-2222-4222-8222-222222222222";
const CHILD: &str = "33333333-3333-4333-8333-333333333333";

fn usage(input: u64, cached: u64, write: u64, output: u64) -> Value {
    json!({
        "input_tokens": input, "cached_input_tokens": cached,
        "cache_write_input_tokens": write, "output_tokens": output,
    })
}

fn meta(thread: &str) -> String {
    json!({"type":"session_meta","payload":{"id":thread,"cli_version":"0.154.0"}}).to_string()
}

/// A child of `PARENT` whose own history starts at `boundary`.
fn child_meta(boundary: u64) -> String {
    json!({"ordinal":0,"type":"session_meta","payload":{
        "id":CHILD,"parent_thread_id":PARENT,"history_mode":"paginated",
        "subagent_history_start_ordinal":boundary,
    }})
    .to_string()
}

fn turn(turn: &str) -> String {
    json!({"type":"turn_context","payload":{"turn_id":turn,"model":"synthetic-model"}}).to_string()
}

fn record(thread: &str, response: &str, usage: &Value) -> String {
    json!({"type":"token_usage_record","payload":{"thread_id":thread,"turn_id":"turn","response_id":response,"usage":usage}})
        .to_string()
}

fn compacted(thread: &str, response: &str, usage: &Value) -> String {
    json!({"type":"compacted","payload":{"message":"","latest_token_usage_record":{
        "thread_id":thread,"response_id":response,"usage":usage,
    }}})
    .to_string()
}

fn counter(total: &Value, last: &Value) -> String {
    json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":total,"last_token_usage":last}}})
        .to_string()
}

/// `line` with an `ordinal`.
fn at(ordinal: u64, line: &str) -> String {
    let mut line: Value = serde_json::from_str(line).expect("synthetic line");
    line["ordinal"] = ordinal.into();
    line.to_string()
}

/// Writes one rollout of `thread`, numbered by `minute` so file names sort.
fn write(root: &tempfile::TempDir, thread: &str, minute: u32, lines: &[String]) {
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).expect("sessions directory");
    fs::write(
        sessions.join(format!("rollout-2026-01-01T00-0{minute}-00-{thread}.jsonl")),
        lines.iter().flat_map(|line| [line.as_str(), "\n"]).collect::<String>(),
    )
    .expect("rollout is writable");
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
            .expect("an inconsistent record never stops ingestion");
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

/// A root rollout of `ROOT` holding `lines`, ingested every way.
fn ingest_root(lines: &[String]) -> Ingested {
    let root = tempfile::tempdir().expect("temporary directory is created");
    write(&root, ROOT, 0, lines);
    ingest_every_way(&root)
}

/// Each counted request's uncached input, cache reads, cache writes and output.
fn counted(ingested: &Ingested) -> Vec<[Option<u64>; 4]> {
    let mut rows: Vec<_> = ingested
        .ledger
        .requests
        .values()
        .filter(|request| request.counting == Counting::Counted)
        .filter_map(|request| request.usage.as_ref())
        .map(|usage| {
            let usage = TokenMeasures::from(usage.revision.usage);
            [usage.uncached_input, usage.cache_read, usage.cache_write_unspecified, usage.output]
        })
        .collect();
    rows.sort();
    rows
}

/// The ledger's diagnostics as code, occurrences and detail.
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

fn thread_id(native: &str) -> urollup_core::ledger::identity::AnalyticalId {
    agent_thread_identity(Agent::Codex, native).expect("thread identity").id
}

/// Each coverage gap's reason and thread.
fn gaps(ingested: &Ingested) -> Vec<(UnobservedReason, Option<String>)> {
    ingested
        .ledger
        .gaps
        .iter()
        .map(|gap| {
            let thread = gap.thread.as_ref().map(ToString::to_string);
            (gap.reason.clone(), thread)
        })
        .collect()
}

fn gap(reason: UnobservedReason, native: &str) -> (UnobservedReason, Option<String>) {
    (reason, Some(thread_id(native).to_string()))
}

fn completeness(ingested: &Ingested, native: &str) -> Completeness {
    selection_totals(&ingested.ledger, &BTreeSet::from([thread_id(native)]))
        .expect("selection totals")
        .completeness
}

fn partial() -> Completeness {
    Completeness::Partial(BTreeSet::from([PartialReason::UnobservedGap]))
}

const UNDERFLOW: &str = "codex-inclusive-input-underflow";

fn root_detail(what: &str, line: u32, excluded: u64) -> String {
    let more = if excluded > 1 {
        format!("; {excluded} records or counter steps of this rollout are excluded")
    } else {
        String::new()
    };
    format!(
        "{what} at rollout-2026-01-01T00-00-00-{ROOT}.jsonl:{line}, so that usage is excluded \
         as a coverage gap{more}"
    )
}

#[test]
fn records_below_their_cache_writes_or_reads_are_excluded_and_the_rest_counts() {
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        // Below reads plus writes: 100 < 60 + 50.
        record(ROOT, "writes", &usage(100, 60, 50, 5)),
        // Below reads alone, which main also refused: 10 < 11.
        record(ROOT, "reads", &usage(10, 11, 0, 1)),
        record(ROOT, "valid", &usage(40, 10, 0, 4)),
    ]);
    assert_eq!(counted(&ingested), [[Some(30), Some(10), Some(0), Some(4)]]);
    assert_eq!(
        diagnostics(&ingested),
        [(
            UNDERFLOW,
            2,
            root_detail(
                "Codex input_tokens 100 is below its 60 cached and 50 cache-write tokens",
                3,
                2
            )
            .as_str(),
        )]
    );
    assert_eq!(gaps(&ingested), [gap(UnobservedReason::InconsistentUsage, ROOT)]);
    assert_eq!(completeness(&ingested, ROOT), partial());
}

#[test]
fn the_diagnostic_names_the_physical_line_after_blank_and_malformed_lines() {
    let ingested = ingest_root(&[
        meta(ROOT),
        String::new(),
        "{\"type\":\"token_usage_record\",".to_owned(),
        record(ROOT, "writes", &usage(100, 60, 50, 5)),
    ]);
    let found = diagnostics(&ingested);
    let underflow: Vec<_> = found.iter().filter(|(code, ..)| *code == UNDERFLOW).collect();
    assert_eq!(underflow.len(), 1, "{found:?}");
    assert_eq!(
        underflow[0].2,
        root_detail(
            "Codex input_tokens 100 is below its 60 cached and 50 cache-write tokens",
            4,
            1
        )
    );
}

#[test]
fn an_inconsistent_cumulative_total_never_lets_earlier_usage_count_again() {
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        counter(&usage(100, 20, 10, 5), &usage(100, 20, 10, 5)),
        // The total cannot be normalized: 150 < 140 + 20.
        counter(&usage(150, 140, 20, 10), &usage(50, 120, 10, 5)),
        // Its delta from the last readable total also covers the inconsistent step, so it
        // does not match its own usage and is excluded too.
        counter(&usage(300, 160, 30, 15), &usage(100, 20, 10, 5)),
        counter(&usage(400, 180, 40, 20), &usage(100, 20, 10, 5)),
    ]);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]; 2]);
    assert_eq!(
        diagnostics(&ingested),
        [(
            UNDERFLOW,
            2,
            root_detail(
                "Codex input_tokens 150 is below its 140 cached and 20 cache-write tokens",
                4,
                2
            )
            .as_str(),
        )]
    );
    assert_eq!(gaps(&ingested), [gap(UnobservedReason::InconsistentUsage, ROOT)]);
}

#[test]
fn a_step_whose_own_usage_is_inconsistent_keeps_its_total_as_the_next_baseline() {
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        counter(&usage(100, 20, 10, 5), &usage(100, 20, 10, 5)),
        // The total is readable; only its last usage is not: 10 < 20.
        counter(&usage(200, 40, 20, 10), &usage(10, 20, 0, 5)),
        counter(&usage(300, 60, 30, 15), &usage(100, 20, 10, 5)),
    ]);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]; 2]);
    let found = diagnostics(&ingested);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), (UNDERFLOW, 1));
    assert_eq!(gaps(&ingested), [gap(UnobservedReason::InconsistentUsage, ROOT)]);
}

#[test]
fn an_inconsistent_first_total_leaves_no_baseline_to_count_from_zero() {
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        // The first total cannot be normalized: 100 < 90 + 20.
        counter(&usage(100, 90, 20, 5), &usage(100, 90, 20, 5)),
        // Counting from zero would charge the inconsistent step's usage again, so the next
        // total is checked against its own usage and excluded.
        counter(&usage(300, 110, 40, 10), &usage(200, 20, 20, 5)),
        counter(&usage(400, 130, 50, 15), &usage(100, 20, 10, 5)),
    ]);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]]);
    let found = diagnostics(&ingested);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), (UNDERFLOW, 2));
    assert_eq!(gaps(&ingested), [gap(UnobservedReason::InconsistentUsage, ROOT)]);
}

#[test]
fn a_refresh_repeating_an_inconsistent_step_is_not_excluded_again() {
    let inconsistent = counter(&usage(200, 40, 20, 10), &usage(10, 20, 0, 5));
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        counter(&usage(100, 20, 10, 5), &usage(100, 20, 10, 5)),
        inconsistent.clone(),
        // Codex re-sends the current totals with every rate-limit refresh.
        inconsistent,
        counter(&usage(300, 60, 30, 15), &usage(100, 20, 10, 5)),
    ]);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]; 2]);
    let found = diagnostics(&ingested);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), (UNDERFLOW, 1), "one step's usage is excluded");
}

#[test]
fn the_counter_of_an_excluded_compacted_original_adds_no_second_exclusion() {
    let compacted_usage = usage(100, 120, 0, 5);
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        counter(&usage(200, 20, 10, 10), &usage(200, 20, 10, 10)),
        // The only record of its response, so the original, and inconsistent: 100 < 120.
        compacted(ROOT, "compacted", &compacted_usage),
        // Its counter reports the same response, so it adds nothing but its total.
        counter(&usage(400, 140, 10, 15), &compacted_usage),
        counter(&usage(500, 160, 20, 20), &usage(100, 20, 10, 5)),
    ]);
    assert_eq!(
        counted(&ingested),
        [[Some(70), Some(20), Some(10), Some(5)], [Some(170), Some(20), Some(10), Some(10)]]
    );
    let found = diagnostics(&ingested);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), (UNDERFLOW, 1));
    assert!(found[0].2.contains(".jsonl:4,"), "{found:?}");
}

#[test]
fn an_unreadable_twin_total_excludes_only_the_step_that_cannot_be_checked() {
    let record_usage = usage(100, 20, 10, 5);
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("t1"),
        record(ROOT, "r1", &record_usage),
        // The record's twin: its usage is the record's, but its total cannot be read.
        counter(&usage(100, 200, 10, 5), &record_usage),
        turn("t2"),
        // A counter-only response that cannot be checked against that total.
        counter(&usage(300, 220, 20, 10), &usage(150, 20, 10, 5)),
    ]);
    assert_eq!(counted(&ingested), [[Some(70), Some(20), Some(10), Some(5)]]);
    assert_eq!(
        diagnostics(&ingested),
        [(
            UNDERFLOW,
            1,
            root_detail(
                "Codex cumulative usage cannot be checked against an inconsistent total before it",
                6,
                1
            )
            .as_str(),
        )]
    );
    assert_eq!(gaps(&ingested), [gap(UnobservedReason::InconsistentUsage, ROOT)]);
}

#[test]
fn a_root_counts_a_lowered_total_after_an_inconsistent_step_as_its_reset_rule_does() {
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        counter(&usage(100, 20, 10, 10), &usage(100, 20, 10, 10)),
        counter(&usage(200, 40, 20, 20), &usage(10, 20, 0, 10)),
        // Codex lowered its running total at compaction; the step's own usage counts.
        counter(&usage(80, 15, 5, 8), &usage(50, 10, 5, 5)),
    ]);
    assert_eq!(
        counted(&ingested),
        [[Some(35), Some(10), Some(5), Some(5)], [Some(70), Some(20), Some(10), Some(10)]]
    );
    let mut codes: Vec<_> = diagnostics(&ingested).iter().map(|(code, n, _)| (*code, *n)).collect();
    codes.sort_unstable();
    assert_eq!(codes, [("codex-counter-epoch-reset", 1), (UNDERFLOW, 1)]);
}

/// A counter-only parent with one counter, and a child that copies it before its boundary
/// and then reports its own step.
fn copied_counter_root(parent_total: &Value, parent_last: &Value, own: [&Value; 2]) -> Ingested {
    let root = tempfile::tempdir().expect("temporary directory is created");
    write(&root, PARENT, 0, &[meta(PARENT), turn("p-turn"), counter(parent_total, parent_last)]);
    write(
        &root,
        CHILD,
        1,
        &[
            child_meta(2),
            at(1, &counter(parent_total, parent_last)),
            at(2, &counter(own[0], own[1])),
        ],
    );
    ingest_every_way(&root)
}

#[test]
fn a_copied_counter_step_with_inconsistent_usage_is_only_evidence() {
    let total = usage(100, 20, 10, 10);
    let own = [&usage(150, 30, 15, 15), &usage(50, 10, 5, 5)];
    // The parent's own step is inconsistent, so the parent loses it; the child's copy of
    // it loses nothing.
    let ingested = copied_counter_root(&total, &usage(10, 20, 0, 10), own);
    let control = copied_counter_root(&total, &usage(100, 20, 10, 10), own);
    assert_eq!(counted(&ingested), [[Some(35), Some(10), Some(5), Some(5)]]);
    assert_eq!(completeness(&ingested, CHILD), Completeness::Complete);
    assert_eq!(completeness(&ingested, PARENT), partial());
    assert_eq!(gaps(&ingested), [gap(UnobservedReason::InconsistentUsage, PARENT)]);
    let found = diagnostics(&ingested);
    assert_eq!(found.len(), 1, "only the parent reports: {found:?}");
    assert_eq!(found[0].0, UNDERFLOW);
    assert!(found[0].2.contains(PARENT), "{found:?}");
    assert_eq!(ingested.ledger.coverage.copies, control.ledger.coverage.copies);
}

/// A parent with one usage record, and a child that reports its own usage records, so
/// its counters are not cumulative, and copies a counter of the parent's before its
/// boundary with `copied_last` as its last usage.
fn copied_counter_in_records(copied_last: &Value) -> Ingested {
    let root = tempfile::tempdir().expect("temporary directory is created");
    let total = usage(100, 20, 10, 10);
    write(&root, PARENT, 0, &[meta(PARENT), turn("turn"), record(PARENT, "parent", &total)]);
    write(
        &root,
        CHILD,
        1,
        &[
            child_meta(2),
            at(1, &counter(&total, copied_last)),
            at(2, &turn("turn")),
            at(3, &record(CHILD, "child", &usage(40, 10, 0, 4))),
        ],
    );
    ingest_every_way(&root)
}

#[test]
fn a_copied_counter_with_inconsistent_usage_beside_records_is_only_evidence() {
    // The copy builder keeps the copy's evidence without its usage; the ingest still
    // completes, and every count equals the control's.
    let ingested = copied_counter_in_records(&usage(10, 20, 0, 10));
    let control = copied_counter_in_records(&usage(100, 20, 10, 10));
    assert_eq!(
        counted(&ingested),
        [[Some(30), Some(10), Some(0), Some(4)], [Some(70), Some(20), Some(10), Some(10)]]
    );
    assert_eq!(counted(&ingested), counted(&control));
    assert_eq!(ingested.ledger.coverage.copies, 2);
    assert_eq!(ingested.ledger.coverage.copies, control.ledger.coverage.copies);
    assert!(diagnostics(&ingested).is_empty(), "{:?}", diagnostics(&ingested));
    assert_eq!(diagnostics(&ingested), diagnostics(&control));
    assert!(gaps(&ingested).is_empty(), "{:?}", gaps(&ingested));
    assert_eq!(gaps(&ingested), gaps(&control));
}

#[test]
fn a_child_step_after_an_unreadable_copied_total_is_excluded_and_reported() {
    let total = usage(100, 120, 0, 10);
    let ingested =
        copied_counter_root(&total, &total, [&usage(150, 130, 5, 15), &usage(50, 10, 5, 5)]);
    assert_eq!(counted(&ingested), [] as [[Option<u64>; 4]; 0]);
    assert_eq!(completeness(&ingested, CHILD), partial());
    let child: Vec<_> = ingested
        .ledger
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.subject == Some(thread_id(CHILD)))
        .collect();
    assert_eq!(child.len(), 1, "{child:?}");
    assert_eq!(child[0].code.token(), UNDERFLOW);
    assert_eq!(
        child[0].detail,
        format!(
            "Codex cumulative usage cannot be checked against an inconsistent total before it \
             at rollout-2026-01-01T00-01-00-{CHILD}.jsonl:3, so that usage is excluded as a \
             coverage gap"
        )
    );
}

#[test]
fn an_unplaced_step_with_inconsistent_usage_is_excluded_once() {
    let ingest = |last: &Value| {
        let root = tempfile::tempdir().expect("temporary directory is created");
        // A counter without an ordinal under a declared boundary cannot be placed.
        write(&root, CHILD, 1, &[child_meta(3), counter(&usage(100, 20, 10, 10), last)]);
        ingest_every_way(&root)
    };
    let ingested = ingest(&usage(10, 20, 0, 10));
    let control = ingest(&usage(100, 20, 10, 10));
    let codes: Vec<_> = diagnostics(&ingested).iter().map(|(code, n, _)| (*code, *n)).collect();
    // The child's parent rollout is not discovered, as in the control.
    assert_eq!(codes, [("codex-history-boundary-unverified", 1), ("thread-orphan", 1)]);
    assert_eq!(gaps(&ingested), [gap(UnobservedReason::UnverifiedHistoryBoundary, CHILD)]);
    assert_eq!(ingested.ledger.coverage.copies, control.ledger.coverage.copies);
}

#[test]
fn a_copy_with_inconsistent_usage_is_only_evidence() {
    let ingested = ingest_root(&[
        meta(ROOT),
        turn("turn"),
        // A record naming another thread is that thread's copy, which never counts here.
        record(PARENT, "copied", &usage(100, 60, 50, 5)),
        record(ROOT, "valid", &usage(40, 10, 0, 4)),
    ]);
    assert_eq!(counted(&ingested), [[Some(30), Some(10), Some(0), Some(4)]]);
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
    assert!(ingested.ledger.gaps.is_empty());
    assert_eq!(ingested.ledger.coverage.copies, 1);
}
