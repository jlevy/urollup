//! Claude Code subagent transcripts copied under another session's directory (uro-wvgc).
//!
//! A resumed or forked session can hold a byte-identical copy of an earlier session's
//! subagent transcript in its own `subagents/` directory, and every line of the copy
//! still names the earlier session. The copy is that session's subagent: its responses
//! count once, owned by one thread, on any worker count and in any discovery order, and
//! still count once after the original is deleted.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use serde_json::Value;
use urollup_core::accounting::totals::{
    Completeness, LedgerTotals, UsageTotals, ledger_totals, selection_totals,
};
use urollup_core::adapters::{Ingested, claude_project};
use urollup_core::ledger::entities::{Counting, Ownership, Relationship, RelationshipKind, Thread};
use urollup_core::ledger::identity::AnalyticalId;
use urollup_core::ledger::reconcile::Ledger;
use urollup_core::selection::{Agent, agent_thread_identity};
use urollup_core::sources::roots::discover;

const ORIGINAL: &str = "00000000-0000-4000-8000-001800000001";
const RESUMED: &str = "00000000-0000-4000-8000-001800000002";
const UNLINKED: &str = "00000000-0000-4000-8000-001800000009";
const COPIED_AGENT: &str = "a0000000000180001";
const NEW_AGENT: &str = "a0000000000180002";
const PROJECT: &str = "projects/-Users-example-project";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/claude-project/resumed-subagent-copy")
}

/// A writable copy of the fixture.
fn fixture_copy() -> tempfile::TempDir {
    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("the copy's directory is created");
        for entry in fs::read_dir(from).expect("the fixture directory is readable") {
            let path = entry.expect("the fixture entry is readable").path();
            let target = to.join(path.file_name().expect("a fixture entry has a name"));
            if path.is_dir() {
                copy_dir(&path, &target);
            } else {
                fs::copy(&path, &target).expect("the fixture file is copied");
            }
        }
    }
    let root = tempfile::tempdir().expect("a temporary root");
    copy_dir(&fixture(), root.path());
    root
}

fn session_file(root: &Path, session: &str) -> PathBuf {
    root.join(PROJECT).join(format!("{session}.jsonl"))
}

fn subagent_file(root: &Path, session: &str, agent: &str) -> PathBuf {
    root.join(PROJECT).join(session).join("subagents").join(format!("agent-{agent}.jsonl"))
}

/// Line `number` (1-based) of a transcript, as a document.
fn line(path: &Path, number: usize) -> Value {
    let text = fs::read_to_string(path).expect("the transcript is readable");
    let line = text.lines().nth(number - 1).expect("the transcript has the line");
    serde_json::from_str(line).expect("the line is a document")
}

fn append_line(path: &Path, value: &Value) {
    let mut text = fs::read_to_string(path).unwrap_or_default();
    text.push_str(&value.to_string());
    text.push('\n');
    fs::write(path, text).expect("the transcript is writable");
}

fn thread_id(native: &str) -> AnalyticalId {
    agent_thread_identity(Agent::Claude, native).expect("a Claude thread identity").id
}

fn subagent_id(session: &str, agent: &str) -> AnalyticalId {
    thread_id(&format!("{session}/{agent}"))
}

fn ingest(root: &Path, workers: usize, reversed: bool) -> Ingested {
    let mut discovery = discover(&[root.to_owned()]);
    if reversed {
        discovery.sources.reverse();
    }
    let workers = NonZeroUsize::new(workers).expect("worker counts are positive");
    claude_project::ingest_discovery_with_workers(discovery, true, workers)
        .expect("the Claude root ingests")
}

/// Everything an ingestion decides, apart from manifest order and capture clocks, which
/// follow discovery.
type Decided = (Ledger, BTreeMap<AnalyticalId, Thread>, Vec<Relationship>);

fn decided(ingested: &Ingested) -> Decided {
    (ingested.ledger.clone(), ingested.threads.clone(), ingested.relationships.clone())
}

/// Ingests `root` on 1 and 8 workers, in discovery order and reversed, and requires the
/// same ledger, threads and relationships from every run; returns the first.
fn ingest_invariantly(root: &Path) -> Ingested {
    let baseline = ingest(root, 1, false);
    for workers in [1, 8] {
        for reversed in [false, true] {
            let run = ingest(root, workers, reversed);
            assert_eq!(
                decided(&run),
                decided(&baseline),
                "{workers} workers, reversed {reversed}: the result differs"
            );
        }
    }
    baseline
}

fn owned_by(ingested: &Ingested, thread: &AnalyticalId) -> UsageTotals {
    let threads = BTreeSet::from([thread.clone()]);
    selection_totals(&ingested.ledger, &threads).expect("selection totals").owned
}

fn totals(ingested: &Ingested) -> LedgerTotals {
    ledger_totals(&ingested.ledger).expect("ledger totals")
}

fn has_edge(ingested: &Ingested, kind: &RelationshipKind, from: &str, to: &AnalyticalId) -> bool {
    let from = thread_id(from);
    ingested
        .relationships
        .iter()
        .any(|edge| &edge.kind == kind && edge.from == from && &edge.to == to)
}

fn diagnostic_codes(ingested: &Ingested) -> BTreeMap<&'static str, u64> {
    let mut codes = BTreeMap::new();
    for diagnostic in &ingested.ledger.diagnostics {
        *codes.entry(diagnostic.code.token()).or_default() += diagnostic.occurrences;
    }
    codes
}

#[test]
fn a_copied_subagent_is_the_original_subagent_on_any_worker_count_and_order() {
    let ingested = ingest_invariantly(&fixture());
    let copied = subagent_id(ORIGINAL, COPIED_AGENT);
    let new = subagent_id(RESUMED, NEW_AGENT);

    let threads: BTreeSet<_> = ingested.threads.keys().cloned().collect();
    let expected: BTreeSet<_> =
        [thread_id(ORIGINAL), thread_id(RESUMED), copied.clone(), new.clone()].into();
    assert_eq!(threads, expected, "the copy adds no thread under the resumed session");
    assert!(!threads.contains(&subagent_id(RESUMED, COPIED_AGENT)));

    let totals = totals(&ingested);
    assert_eq!((totals.total.requests, totals.owned.requests), (7, 7));
    assert_eq!(totals.ambiguous.requests, 0, "a copy never makes its responses ambiguous");
    assert_eq!(totals.completeness, Completeness::Complete);
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
    assert_eq!(ingested.ledger.coverage.copies, 2, "only the resumed session's replays copy");

    let subagent = owned_by(&ingested, &copied);
    assert_eq!(subagent.requests, 2);
    assert_eq!(subagent.tokens.output, Some(65));
    for request in ingested.ledger.requests.values() {
        if request.ownership == (Ownership::Owned { thread: copied.clone() }) {
            assert_eq!(request.evidence().len(), 2, "the original and its copy are revisions");
            assert!(request.copies().is_empty());
        }
    }
    assert_eq!(owned_by(&ingested, &new).requests, 1, "the resumed session's own subagent");
    assert_eq!(owned_by(&ingested, &thread_id(RESUMED)).requests, 2);

    assert_eq!(ingested.relationships.len(), 3);
    assert!(has_edge(&ingested, &RelationshipKind::Spawn, ORIGINAL, &copied));
    assert!(has_edge(&ingested, &RelationshipKind::Spawn, RESUMED, &new));
    assert!(has_edge(&ingested, &RelationshipKind::Fork, ORIGINAL, &thread_id(RESUMED)));
}

#[test]
fn a_copy_still_counts_once_after_claude_code_deletes_the_original() {
    let root = fixture_copy();
    fs::remove_file(session_file(root.path(), ORIGINAL)).expect("the original is removed");
    fs::remove_dir_all(root.path().join(PROJECT).join(ORIGINAL))
        .expect("the original's subagents are removed");
    let ingested = ingest_invariantly(root.path());
    let copied = subagent_id(ORIGINAL, COPIED_AGENT);

    let subagent = owned_by(&ingested, &copied);
    assert_eq!(subagent.requests, 2, "the copy alone counts the subagent's responses");
    assert_eq!(subagent.tokens.output, Some(65));
    for request in ingested.ledger.requests.values() {
        if request.ownership == (Ownership::Owned { thread: copied.clone() }) {
            assert_eq!(request.counting, Counting::Counted);
            assert_eq!(request.evidence().len(), 1);
        }
    }
    assert!(has_edge(&ingested, &RelationshipKind::Spawn, ORIGINAL, &copied));
    assert!(!ingested.threads.contains_key(&subagent_id(RESUMED, COPIED_AGENT)));

    // The resumed session's replays of the deleted session's own responses stay copies,
    // as design §3.4 treats every resumed replay.
    let totals = totals(&ingested);
    assert_eq!((totals.total.requests, totals.owned.requests), (5, 5));
    assert_eq!(totals.copy_only.requests, 2);
    assert_eq!(
        diagnostic_codes(&ingested),
        BTreeMap::from([("claude-nested-copy-without-original", 2)])
    );
}

#[test]
fn a_continued_copy_belongs_line_by_line_to_the_session_each_line_names() {
    let root = fixture_copy();
    let copy = subagent_file(root.path(), RESUMED, COPIED_AGENT);
    let mut continued = line(&copy, 3);
    continued["sessionId"] = RESUMED.into();
    continued["parentUuid"] = continued["uuid"].clone();
    continued["uuid"] = "00000000-0000-4000-a000-001800000103".into();
    continued["timestamp"] = "2026-09-06T11:00:10.000Z".into();
    continued["requestId"] = "req_011Synthetic18X3".into();
    continued["message"]["id"] = "msg_01Synthetic18X3".into();
    append_line(&copy, &continued);
    let ingested = ingest_invariantly(root.path());

    let copied = subagent_id(ORIGINAL, COPIED_AGENT);
    let continuation = subagent_id(RESUMED, COPIED_AGENT);
    assert_eq!(owned_by(&ingested, &copied).requests, 2, "the copied lines stay the original's");
    assert_eq!(owned_by(&ingested, &continuation).requests, 1, "the new line is the resumed's");
    assert!(has_edge(&ingested, &RelationshipKind::Spawn, RESUMED, &continuation));
    let totals = totals(&ingested);
    assert_eq!((totals.total.requests, totals.owned.requests), (8, 8));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);
}

#[test]
fn unlinked_sessions_claiming_one_response_stay_ambiguous_beside_a_copy() {
    let root = fixture_copy();
    let mut claim = line(&session_file(root.path(), RESUMED), 8);
    claim["sessionId"] = UNLINKED.into();
    claim["parentUuid"] = Value::Null;
    claim["uuid"] = "00000000-0000-4000-a000-001800000901".into();
    append_line(&session_file(root.path(), UNLINKED), &claim);
    let ingested = ingest_invariantly(root.path());

    let totals = totals(&ingested);
    assert_eq!((totals.total.requests, totals.owned.requests), (7, 6));
    assert_eq!(totals.ambiguous.requests, 1);
    let ambiguous: Vec<_> = ingested
        .ledger
        .requests
        .values()
        .filter_map(|request| match &request.ownership {
            Ownership::Ambiguous { candidates } => Some(candidates.clone()),
            Ownership::Owned { .. } | Ownership::Unknown => None,
        })
        .collect();
    assert_eq!(ambiguous, [BTreeSet::from([thread_id(RESUMED), thread_id(UNLINKED)])]);
    // One diagnostic, citing each session's record.
    assert_eq!(ingested.ledger.diagnostics.len(), 1);
    assert_eq!(diagnostic_codes(&ingested), BTreeMap::from([("conflicting-owners", 2)]));
    assert_eq!(owned_by(&ingested, &subagent_id(ORIGINAL, COPIED_AGENT)).requests, 2);
}
