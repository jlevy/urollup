//! Claude Code subagent records copied under another thread (uro-wvgc).
//!
//! A resumed or forked session can hold a byte-identical copy of an earlier session's
//! subagent transcript in its own `subagents/` directory, and every line of the copy
//! still names the earlier session. The copy is that session's subagent: its responses
//! count once, owned by one thread, on any worker count and in any discovery order, and
//! still count once after the original is deleted. A `progress` record's nested copy of a
//! subagent record never decides a spawn either, nor does a fork's replay of the record
//! that spawned it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use urollup_core::accounting::totals::{
    Completeness, LedgerTotals, UsageTotals, ledger_totals, selection_totals,
};
use urollup_core::adapters::{Ingested, claude_project};
use urollup_core::ledger::entities::{Counting, Ownership, Relationship, RelationshipKind, Thread};
use urollup_core::ledger::identity::AnalyticalId;
use urollup_core::ledger::reconcile::Ledger;
use urollup_core::selection::{Agent, SelectionQuery, SessionIndex, agent_thread_identity};
use urollup_core::sources::roots::discover;

const CASE: &str = "resumed-subagent-copy";
const ORIGINAL: &str = "00000000-0000-4000-8000-001800000001";
const RESUMED: &str = "00000000-0000-4000-8000-001800000002";
const UNLINKED: &str = "00000000-0000-4000-8000-001800000009";
const COPIED_AGENT: &str = "a0000000000180001";
const NEW_AGENT: &str = "a0000000000180002";

const WORKFLOW_CASE: &str = "workflow-subagents";
const WORKFLOW: &str = "00000000-0000-4000-8000-001100000001";
const WORKFLOW_RESUMED: &str = "00000000-0000-4000-8000-001100000009";
const WORKFLOW_AGENT: &str = "a0000000000110001";
const WORKFLOW_STEP: &str = "a0000000000110002";

const SIDE_QUESTION_CASE: &str = "btw-side-question";
const SIDE_QUESTION: &str = "00000000-0000-4000-8000-000400000001";
const SIDE_QUESTION_RESUMED: &str = "00000000-0000-4000-8000-000400000009";

const FORK_CASE: &str = "fork-subagent-uuid-replay";
const FORK_SESSION: &str = "00000000-0000-4000-8000-000500000001";
const FORK_AGENT: &str = "a0000000000050001";

const PROJECT: &str = "projects/-Users-example-project";

fn fixture(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude-project").join(case)
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the copy's directory is created");
    for entry in fs::read_dir(from).expect("the directory is readable") {
        let path = entry.expect("the directory entry is readable").path();
        let target = to.join(path.file_name().expect("a directory entry has a name"));
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            fs::copy(&path, &target).expect("the file is copied");
        }
    }
}

/// A writable copy of a fixture case.
fn case_copy(case: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary root");
    copy_dir(&fixture(case), root.path());
    root
}

fn session_file(root: &Path, session: &str) -> PathBuf {
    root.join(PROJECT).join(format!("{session}.jsonl"))
}

fn session_dir(root: &Path, session: &str) -> PathBuf {
    root.join(PROJECT).join(session)
}

fn subagent_file(root: &Path, session: &str, agent: &str) -> PathBuf {
    session_dir(root, session).join("subagents").join(format!("agent-{agent}.jsonl"))
}

/// Resumes `session` as `resumed` the way a forking resume leaves it: the resumed file
/// replays the session's transcript byte for byte, under its `sessionId`, and its
/// `subagents/` directory holds byte copies of the session's subagent transcripts and
/// sidecars.
fn resume(root: &Path, session: &str, resumed: &str) {
    fs::copy(session_file(root, session), session_file(root, resumed))
        .expect("the resumed session replays the transcript");
    copy_dir(&session_dir(root, session), &session_dir(root, resumed));
}

/// Removes a session's transcript and subagents, as Claude Code's cleanup does.
fn delete_session(root: &Path, session: &str) {
    fs::remove_file(session_file(root, session)).expect("the transcript is removed");
    fs::remove_dir_all(session_dir(root, session)).expect("the subagents are removed");
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

/// The parents of `child` through edges of `kind`.
fn parents(
    ingested: &Ingested,
    kind: &RelationshipKind,
    child: &AnalyticalId,
) -> Vec<AnalyticalId> {
    ingested
        .relationships
        .iter()
        .filter(|edge| &edge.kind == kind && &edge.to == child)
        .map(|edge| edge.from.clone())
        .collect()
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
    let ingested = ingest_invariantly(&fixture(CASE));
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
    assert_eq!(parents(&ingested, &RelationshipKind::Spawn, &copied), [thread_id(ORIGINAL)]);
    assert_eq!(parents(&ingested, &RelationshipKind::Spawn, &new), [thread_id(RESUMED)]);
    assert_eq!(
        parents(&ingested, &RelationshipKind::Fork, &thread_id(RESUMED)),
        [thread_id(ORIGINAL)]
    );
}

#[test]
fn a_copy_still_counts_once_after_claude_code_deletes_the_original() {
    let root = case_copy(CASE);
    delete_session(root.path(), ORIGINAL);
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
    assert_eq!(parents(&ingested, &RelationshipKind::Spawn, &copied), [thread_id(ORIGINAL)]);
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

/// Continues the resumed session's copy of the subagent transcript with one line that
/// names the resumed session, and returns the copy's path.
fn continue_copy(root: &Path) -> PathBuf {
    let copy = subagent_file(root, RESUMED, COPIED_AGENT);
    let mut continued = line(&copy, 3);
    continued["sessionId"] = RESUMED.into();
    continued["parentUuid"] = continued["uuid"].clone();
    continued["uuid"] = "00000000-0000-4000-a000-001800000103".into();
    continued["timestamp"] = "2026-09-06T11:00:10.000Z".into();
    continued["requestId"] = "req_011Synthetic18X3".into();
    continued["message"]["id"] = "msg_01Synthetic18X3".into();
    append_line(&copy, &continued);
    copy
}

#[test]
fn a_continued_copy_belongs_line_by_line_to_the_session_each_line_names() {
    let root = case_copy(CASE);
    let copy = continue_copy(root.path());
    let ingested = ingest_invariantly(root.path());

    let copied = subagent_id(ORIGINAL, COPIED_AGENT);
    let continuation = subagent_id(RESUMED, COPIED_AGENT);
    assert_eq!(owned_by(&ingested, &copied).requests, 2, "the copied lines stay the original's");
    assert_eq!(owned_by(&ingested, &continuation).requests, 1, "the new line is the resumed's");
    assert_eq!(parents(&ingested, &RelationshipKind::Spawn, &continuation), [thread_id(RESUMED)]);
    let totals = totals(&ingested);
    assert_eq!((totals.total.requests, totals.owned.requests), (8, 8));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);

    // The copy's path holds records of both threads and names the one its location
    // implies, as a hook's transcript path does; the original's path names the original.
    let mut index = SessionIndex::default();
    index.add(Agent::Claude, &ingested).expect("the sessions index");
    assert_eq!(index.resolve(copy.as_os_str()).expect("the copy's path resolves"), continuation);
    let original = subagent_file(root.path(), ORIGINAL, COPIED_AGENT);
    assert_eq!(index.resolve(original.as_os_str()).expect("the original resolves"), copied);
}

#[test]
fn a_transcript_path_names_its_location_below_a_directory_named_subagents() {
    // The root lies below a directory named `subagents`, which is not the `subagents/`
    // directory of a session.
    let parent = tempfile::tempdir().expect("a temporary directory");
    let root = parent.path().join("subagents/claude");
    copy_dir(&fixture(CASE), &root);
    let copy = continue_copy(&root);
    let ingested = ingest(&root, 1, false);

    let mut index = SessionIndex::default();
    index.add(Agent::Claude, &ingested).expect("the sessions index");
    assert_eq!(
        index.resolve(copy.as_os_str()).expect("the copy's path resolves"),
        subagent_id(RESUMED, COPIED_AGENT)
    );
}

#[test]
fn unlinked_sessions_claiming_one_response_stay_ambiguous_beside_a_copy() {
    let root = case_copy(CASE);
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

#[test]
fn a_divergent_copied_sidecar_never_gives_a_subagent_a_second_parent() {
    let root = case_copy(CASE);
    let sidecar = subagent_file(root.path(), RESUMED, COPIED_AGENT).with_extension("meta.json");
    let mut meta: Value =
        serde_json::from_slice(&fs::read(&sidecar).expect("the sidecar is readable"))
            .expect("the sidecar is a document");
    meta["toolUseId"] = "toolu_01Synthetic18SpawnY".into();
    fs::write(&sidecar, format!("{meta}\n")).expect("the sidecar is writable");
    let ingested = ingest_invariantly(root.path());

    let copied = subagent_id(ORIGINAL, COPIED_AGENT);
    assert_eq!(
        parents(&ingested, &RelationshipKind::Spawn, &copied).len(),
        1,
        "the lowest-src- sidecar whose tool use has an issuer names the one parent"
    );
    assert_eq!(totals(&ingested).owned.requests, 7);
}

#[test]
fn a_nested_copy_of_a_spawning_record_never_decides_the_spawn() {
    let root = case_copy(WORKFLOW_CASE);
    // The main transcript carries a progress copy of the subagent's spawning record.
    let spawning = line(&subagent_file(root.path(), WORKFLOW, WORKFLOW_AGENT), 2);
    let progress = json!({
        "parentUuid": null,
        "isSidechain": false,
        "userType": "external",
        "cwd": "/Users/example/project",
        "sessionId": WORKFLOW,
        "version": "2.1.214",
        "gitBranch": "main",
        "type": "progress",
        "data": {"message": spawning},
        "uuid": "00000000-0000-4000-a000-001100000090",
        "timestamp": "2026-09-01T10:00:05.001Z"
    });
    append_line(&session_file(root.path(), WORKFLOW), &progress);
    let ingested = ingest_invariantly(root.path());

    let step = subagent_id(WORKFLOW, WORKFLOW_STEP);
    assert_eq!(
        parents(&ingested, &RelationshipKind::Spawn, &step),
        [subagent_id(WORKFLOW, WORKFLOW_AGENT)],
        "the subagent whose own transcript issued the tool use spawned the step"
    );
    assert_eq!(totals(&ingested).owned.requests, 5);
}

#[test]
fn nested_subagents_copied_into_a_resumed_session_stay_one_tree() {
    let root = case_copy(WORKFLOW_CASE);
    resume(root.path(), WORKFLOW, WORKFLOW_RESUMED);
    let agent = subagent_id(WORKFLOW, WORKFLOW_AGENT);
    let step = subagent_id(WORKFLOW, WORKFLOW_STEP);
    let tree = |ingested: &Ingested| {
        assert_eq!(parents(ingested, &RelationshipKind::Spawn, &agent), [thread_id(WORKFLOW)]);
        assert_eq!(
            parents(ingested, &RelationshipKind::Spawn, &step),
            std::slice::from_ref(&agent)
        );
        assert_eq!(owned_by(ingested, &agent).requests, 2);
        assert_eq!(owned_by(ingested, &step).requests, 1);
        assert!(!ingested.threads.contains_key(&subagent_id(WORKFLOW_RESUMED, WORKFLOW_AGENT)));
        assert!(!ingested.threads.contains_key(&subagent_id(WORKFLOW_RESUMED, WORKFLOW_STEP)));
        assert_eq!(totals(ingested).ambiguous.requests, 0);
    };

    let ingested = ingest_invariantly(root.path());
    tree(&ingested);
    let totals_present = totals(&ingested);
    assert_eq!((totals_present.total.requests, totals_present.owned.requests), (5, 5));
    assert!(ingested.ledger.diagnostics.is_empty(), "{:?}", ingested.ledger.diagnostics);

    // After the original session is deleted, the copies still count each subagent once;
    // its own responses survive only as the resumed session's replays.
    delete_session(root.path(), WORKFLOW);
    let ingested = ingest_invariantly(root.path());
    tree(&ingested);
    let totals_deleted = totals(&ingested);
    assert_eq!((totals_deleted.total.requests, totals_deleted.owned.requests), (3, 3));
    assert_eq!(totals_deleted.copy_only.requests, 2);
}

#[test]
fn a_resumed_side_question_copy_keeps_one_owner_after_its_session_is_deleted() {
    let root = case_copy(SIDE_QUESTION_CASE);
    resume(root.path(), SIDE_QUESTION, SIDE_QUESTION_RESUMED);
    delete_session(root.path(), SIDE_QUESTION);
    let ingested = ingest_invariantly(root.path());

    let totals = totals(&ingested);
    assert_eq!(totals.ambiguous.requests, 0, "the replay takes the side question's owner");
    assert_eq!((totals.total.requests, totals.owned.requests), (2, 2));
    // The side question's record of the parent's response is now its only original.
    assert_eq!(totals.total.tokens.cache_read, Some(50_700));
    assert_eq!(totals.total.tokens.output, Some(40));
    assert!(
        !diagnostic_codes(&ingested).contains_key("conflicting-owners"),
        "{:?}",
        ingested.ledger.diagnostics
    );
}

#[test]
fn a_fork_replaying_the_record_that_spawned_it_never_spawns_itself() {
    let root = case_copy(FORK_CASE);
    let sidecar = subagent_file(root.path(), FORK_SESSION, FORK_AGENT).with_extension("meta.json");
    let meta = json!({"agentType": "fork", "toolUseId": "toolu_01Synthetic05Fork"});
    fs::write(&sidecar, format!("{meta}\n")).expect("the sidecar is writable");
    let fork = subagent_id(FORK_SESSION, FORK_AGENT);
    let session = thread_id(FORK_SESSION);

    // The fork replays its parent's records by `uuid`, including the one that issued the
    // fork's tool use; the session's own record of it, first by owner precedence, decides
    // the spawn in every discovery order.
    let ingested = ingest_invariantly(root.path());
    assert_eq!(parents(&ingested, &RelationshipKind::Spawn, &fork), std::slice::from_ref(&session));
    assert_eq!(totals(&ingested).owned.requests, 3);
    let mut index = SessionIndex::default();
    index.add(Agent::Claude, &ingested).expect("the sessions index");
    let query = SelectionQuery { sessions: vec![FORK_SESSION.into()], ..SelectionQuery::default() };
    assert_eq!(
        index.select(&query).expect("the session selects"),
        [session.clone(), fork.clone()].into()
    );

    // Once the session's transcript is deleted, the fork's replay is the only record of
    // the tool use, and a thread never spawns itself: its session does.
    fs::remove_file(session_file(root.path(), FORK_SESSION)).expect("the transcript is removed");
    let ingested = ingest_invariantly(root.path());
    assert_eq!(parents(&ingested, &RelationshipKind::Spawn, &fork), [session]);
}
