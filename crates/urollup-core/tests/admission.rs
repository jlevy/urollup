//! Configured admission limits apply across worker results before reconciliation.

use std::fs;
use std::mem::size_of;
use std::num::NonZeroUsize;

use serde_json::json;
use urollup_core::adapters::{AdapterError, Decoding, claude_project, codex_rollout, worker_slot};
use urollup_core::ledger::admission::{Component, Hold, MemoryAdmission, ProcessModel, model};
use urollup_core::ledger::capacity::{MemoryBudget, ObservationCapacity};
use urollup_core::ledger::reconcile::{ReconcileError, RequestObservation};
use urollup_core::sources::roots::discover;

/// Five distinct requests across two files, including the Codex pending-counter path.
fn corpus(dialect: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("synthetic source directory");
    for (file, rows) in [(1, 2), (2, 3)] {
        let thread = format!("00000000-0000-4000-8000-{file:012}");
        let mut lines = String::new();
        if dialect != "claude" {
            lines.push_str(&json!({"type":"session_meta","payload":{"id":thread}}).to_string());
            lines.push('\n');
        }
        for row in 0..rows {
            let record = match dialect {
                "claude" => json!({
                    "type":"assistant", "sessionId":thread,
                    "uuid":format!("record-{file}-{row}"),
                    "message":{"id":format!("message-{file}-{row}"),
                        "usage":{"input_tokens":1,"output_tokens":1}}
                }),
                "codex-direct" => json!({
                    "type":"token_usage_record", "payload":{
                        "thread_id":thread,"response_id":format!("response-{file}-{row}"),
                        "usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}
                }),
                "codex-counter" => json!({
                    "type":"event_msg", "payload":{"type":"token_count", "info":{
                        "total_token_usage":{"input_tokens":row+1,"output_tokens":row+1,"total_tokens":2*(row+1)},
                        "last_token_usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}
                }),
                _ => panic!("unknown test dialect"),
            };
            lines.push_str(&record.to_string());
            lines.push('\n');
        }
        fs::write(root.path().join(format!("{thread}.jsonl")), lines)
            .expect("synthetic transcript is written");
    }
    root
}

#[test]
fn configured_capacity_is_shared_before_direct_or_pending_rows_are_retained() {
    let mut late_refusals = Vec::new();
    for dialect in ["claude", "codex-direct", "codex-counter"] {
        let root = corpus(dialect);
        let ingest = if dialect == "claude" {
            claude_project::ingest_discovery_with_capacity
        } else {
            codex_rollout::ingest_discovery_with_capacity
        };
        let byte_budget = ObservationCapacity::from_byte_budget(
            u64::try_from(size_of::<RequestObservation>()).expect("row size fits u64"),
            "test byte budget",
        );
        assert_eq!(byte_budget.maximum(), 1);
        for count in [1, 2, 8] {
            let workers = NonZeroUsize::new(count).expect("positive workers");
            // Five slots must admit exactly five rows, with no second charge when
            // Codex direct rows become observations on their decoding worker.
            let ingested = ingest(
                discover(&[root.path().to_owned()]),
                true,
                workers,
                &ObservationCapacity::from_rows(5),
            )
            .expect("the exact capacity succeeds");
            assert_eq!(ingested.ledger.coverage.observations, 5, "{dialect}/{count}");
            assert_eq!(ingested.ledger.requests.len(), 5, "{dialect}/{count}");

            for capacity in [ObservationCapacity::from_rows(1), byte_budget.clone()] {
                let error = ingest(discover(&[root.path().to_owned()]), true, workers, &capacity)
                    .expect_err("the second retained row exceeds the shared budget");
                let AdapterError::Reconcile(ReconcileError::CapacityExceeded {
                    observations,
                    maximum,
                    limit,
                }) = error
                else {
                    panic!("{dialect}/{count}: unexpected error {error}");
                };
                assert_eq!(maximum, 1, "{dialect}/{count}");
                assert_eq!(limit, capacity.label(), "{dialect}/{count}");
                if observations != 2 {
                    late_refusals
                        .push(format!("{dialect}/{count}/{limit}: got {observations}, expected 2"));
                }
            }
        }
    }
    assert!(late_refusals.is_empty(), "late refusals: {late_refusals:#?}");
}

/// A zstd frame of `content` with a 1 KiB window and its content size, which declares a
/// window of `2^(10 + exponent)` instead: a larger declared window never makes a valid
/// frame invalid, and a frame larger than its window keeps its window descriptor at byte 5.
fn frame(content: &[u8], exponent: u8) -> Vec<u8> {
    use std::io::Write as _;
    assert!(content.len() > 1024, "a frame larger than its window is multi-segment");
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 3).expect("an encoder");
    encoder.window_log(10).expect("a 1 KiB window");
    encoder.include_contentsize(true).expect("a content size");
    encoder.set_pledged_src_size(Some(content.len() as u64)).expect("a pledged size");
    encoder.write_all(content).expect("the frame encodes");
    let mut frame = encoder.finish().expect("the frame ends");
    frame[5] = exponent << 3;
    frame
}

#[test]
fn a_wide_second_frame_decodes_in_one_pass_and_rows_stay_exact() {
    let root = tempfile::tempdir().expect("synthetic source directory");
    let thread = "00000000-0000-4000-8000-000000000001";
    let line = |record: serde_json::Value| format!("{record}\n");
    let usage = |row: usize| {
        line(json!({"type":"token_usage_record", "payload":{
            "thread_id":thread, "response_id":format!("response-{row}"),
            "note":"x".repeat(400),
            "usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}))
    };
    let head =
        [line(json!({"type":"session_meta","payload":{"id":thread}})), usage(0), usage(1)].concat();
    let tail = [usage(2), usage(3), usage(4)].concat();
    let plain = root.path().join("plain");
    let compressed = root.path().join("compressed");
    for directory in [&plain, &compressed] {
        fs::create_dir_all(directory).expect("a source root");
    }
    fs::write(plain.join(format!("{thread}.jsonl")), [&head[..], &tail[..]].concat())
        .expect("the plain twin is written");
    // The second frame declares a 16 MiB window, past the worker slot's 8 MiB.
    let frames = [frame(head.as_bytes(), 0), frame(tail.as_bytes(), 14)].concat();
    fs::write(compressed.join(format!("{thread}.jsonl.zst")), frames)
        .expect("the two-frame rollout is written");

    let ingest = |root: &std::path::Path, workers: usize, rows: usize| {
        codex_rollout::ingest_discovery_with_capacity(
            discover(&[root.to_owned()]),
            true,
            NonZeroUsize::new(workers).expect("positive workers"),
            &ObservationCapacity::from_rows(rows),
        )
    };
    let expected = ingest(&plain, 1, 5).expect("the plain twin reads");
    for workers in [1, 8] {
        // No frame is decoded twice, so exactly five rows are reserved.
        let ingested = ingest(&compressed, workers, 5).expect("the exact row count succeeds");
        assert!(ingested.manifest.entries.iter().all(|entry| entry.failures.is_empty()));
        assert_eq!(ingested.ledger.coverage.observations, 5);
        assert_eq!(ingested.ledger.requests, expected.ledger.requests);
        let error = ingest(&compressed, workers, 4).expect_err("four rows are too few");
        assert!(
            matches!(
                error,
                AdapterError::Reconcile(ReconcileError::CapacityExceeded {
                    observations: 5,
                    maximum: 4,
                    ..
                })
            ),
            "{error}"
        );
    }
}

/// Each fixture case of `dialect`, with the roots its adapter reads.
fn fixture_cases(dialect: &str) -> Vec<(String, Vec<std::path::PathBuf>)> {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(dialect);
    let mut cases: Vec<_> = fs::read_dir(root)
        .expect("the fixture directory is readable")
        .map(|entry| entry.expect("a fixture entry").path())
        .filter(|path| path.is_dir())
        .map(|path| {
            let name = path.file_name().expect("a case name").to_string_lossy().into_owned();
            let roots = if dialect == "codex-rollout" {
                codex_rollout::rollout_roots(&[path])
            } else {
                vec![path]
            };
            (name, roots)
        })
        .collect();
    cases.sort();
    cases
}

type IngestWithAdmission = fn(
    urollup_core::sources::roots::Discovery,
    bool,
    &Decoding<'_>,
) -> Result<urollup_core::adapters::Ingested, AdapterError>;

const DIALECTS: [(&str, IngestWithAdmission); 2] = [
    ("claude-project", claude_project::ingest_discovery_with_admission),
    ("codex-rollout", codex_rollout::ingest_discovery_with_admission),
];

/// Two roots holding the same transcript or rollout, so both sources share one source ID:
/// in the first root a later record declares a long dialect version, and the second root
/// holds only the first record, so its source row copies a version it never declared.
fn shared_source_ids(dialect: &str) -> (tempfile::TempDir, Vec<std::path::PathBuf>) {
    let root = tempfile::tempdir().expect("synthetic source directory");
    let version = "9".repeat(206);
    let (path, first, later) = if dialect == "claude-project" {
        let session = "00000000-0000-4000-8000-000000000001";
        (
            format!("-Users-example-project/{session}.jsonl"),
            json!({"type":"summary","summary":"a session","leafUuid":"leaf"}),
            json!({"type":"user","sessionId":session,"uuid":"u1","version":version,
                "cwd":"/work/app","message":{"role":"user","content":"hello"}}),
        )
    } else {
        let thread = "019f0000-0000-7000-8000-000000000001";
        (
            format!("rollout-2026-09-02T17-00-00-{thread}.jsonl"),
            json!({"type":"session_meta","payload":{"id":thread}}),
            json!({"type":"session_meta","payload":{"id":thread,"cli_version":version}}),
        )
    };
    let roots: Vec<_> = ["a", "b"].iter().map(|name| root.path().join(name)).collect();
    for (index, directory) in roots.iter().enumerate() {
        let file = directory.join(&path);
        fs::create_dir_all(file.parent().expect("a parent")).expect("a source directory");
        let mut text = format!("{first}\n");
        if index == 0 {
            text.push_str(&later.to_string());
            text.push('\n');
        }
        fs::write(file, text).expect("the source is written");
    }
    (root, roots)
}

#[test]
fn source_rows_cover_every_sources_manifest_entry_and_source_row() {
    use urollup_core::ledger::admission::deep_size::DeepSize as _;
    use urollup_core::ledger::entities::Basis;
    for (dialect, ingest) in DIALECTS {
        let (_shared, shared_roots) = shared_source_ids(dialect);
        let cases = fixture_cases(dialect)
            .into_iter()
            .chain([("shared source ID".to_owned(), shared_roots)]);
        for (case, roots) in cases {
            let discovery = discover(&roots);
            let slot = worker_slot(&[&discovery]);
            let admission = MemoryAdmission::unlimited();
            let workers = NonZeroUsize::new(2).expect("positive workers");
            let ingested =
                ingest(discovery, true, &Decoding { admission: &admission, workers, slot })
                    .expect("the case ingests");
            if case == "shared source ID" {
                let ids: Vec<_> =
                    ingested.manifest.entries.iter().map(|entry| &entry.source).collect();
                assert!(ids.len() == 2 && ids[0] == ids[1], "{dialect}: one source ID");
                assert!(
                    ingested.sources.iter().all(|row| matches!(&row.dialect_version,
                        Basis::Observed(version) if version.len() == 206)),
                    "{dialect}: both rows copy the declared version"
                );
            }
            // The join's copies are the harness's to check; here, what the rows hold.
            let sources = ingested.manifest.entries.len() as u64;
            let rows = admission.charged_by(Component::Sources) - sources * model::join_copies();
            let held = ingested.manifest.entries.heap() + ingested.sources.heap();
            assert!(rows >= held, "{dialect}/{case}: {rows} charged for {held} held");
        }
    }
}

#[test]
fn fewer_slots_decode_on_fewer_threads_with_the_same_result() {
    for (dialect, ingest) in DIALECTS {
        for (case, roots) in fixture_cases(dialect) {
            let slot = worker_slot(&[&discover(&roots)]);
            let run = |admission: &MemoryAdmission| {
                admission.raise_hold(Hold::Coordinating, slot).expect("the coordinating slot fits");
                let workers = NonZeroUsize::new(8).expect("positive workers");
                ingest(discover(&roots), true, &Decoding { admission, workers, slot })
                    .expect("the fixture ingests")
            };
            let unlimited = run(&MemoryAdmission::unlimited());
            // ⌊B / (8 × 1.5 × b)⌋ is one slot: a single decoding thread.
            let one_slot = MemoryAdmission::new(
                MemoryBudget::exact(12 * slot - 1, "one slot"),
                ProcessModel::DEFAULT,
            );
            assert_eq!(one_slot.worker_slots(slot, NonZeroUsize::new(8)).get(), 1);
            let narrow = run(&one_slot);
            assert_eq!(narrow.ledger, unlimited.ledger, "{dialect}/{case}");
            assert_eq!(narrow.threads, unlimited.threads, "{dialect}/{case}");
        }
    }
}

/// A Codex home of `families` root rollouts of `records` token counts each, every one
/// after a line of `padding` bytes of content the adapter skips, and a legacy fork of
/// each root that copies the root's first half before counting on its own; plus one
/// rollout that reports usage records. The rate limits change on every token count when
/// `distinct_limits`, and every tenth turn names a model of its own when
/// `distinct_models`; otherwise every count repeats one snapshot and one model.
fn codex_families(
    families: usize,
    records: usize,
    padding: usize,
    distinct_limits: bool,
    distinct_models: bool,
) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a synthetic Codex home");
    let sessions = root.path().join("sessions/2026/09/02");
    fs::create_dir_all(&sessions).expect("a sessions directory");
    let write = |thread: &str, lines: &[serde_json::Value]| {
        let text: String = lines.iter().map(|line| line.to_string() + "\n").collect();
        fs::write(sessions.join(format!("rollout-2026-09-02T17-00-00-{thread}.jsonl")), text)
            .expect("a rollout is written");
    };
    let turn = |family: usize, record: usize| {
        let model =
            if distinct_models { format!("gpt-{family}-{record}") } else { "gpt-test".to_owned() };
        json!({"type": "turn_context",
            "payload": {"turn_id": format!("turn-{family}-{record}"), "model": model, "effort": "high"}})
    };
    let count = |record: usize, total: usize| {
        let used = if distinct_limits { record } else { 1 };
        json!({"timestamp": "2026-09-02T17:00:02.000Z", "type": "event_msg",
            "payload": {"type": "token_count",
                "info": {"total_token_usage": {"input_tokens": total * 100, "output_tokens": total * 10,
                    "total_tokens": total * 110},
                    "last_token_usage": {"input_tokens": 100, "output_tokens": 10, "total_tokens": 110}},
                "rate_limits": {"limit_id": "codex",
                    "primary": {"used_percent": used, "window_minutes": 300},
                    "secondary": {"used_percent": 1, "window_minutes": 10080}}}})
    };
    let content = json!({"type": "response_item",
        "payload": {"type": "message", "content": [{"type": "output_text", "text": "x".repeat(padding)}]}});
    for family in 0..families {
        let parent = format!("019f0000-0000-7000-8000-{family:012}");
        let child = format!("019f0000-0000-7000-8000-{:012}", 1_000 + family);
        let meta = |id: &str| {
            json!({"type": "session_meta", "payload": {"id": id, "cwd": "/home/example/project",
                "cli_version": "0.150.0", "source": "cli", "model_provider": "openai"}})
        };
        let mut lines = vec![meta(&parent)];
        let mut copied = Vec::new();
        for record in 0..records {
            if record % 10 == 0 {
                lines.push(turn(family, record));
            }
            lines.push(content.clone());
            lines.push(count(record, record + 1));
            if record < records / 2 {
                copied.clone_from(&lines);
            }
        }
        write(&parent, &lines);
        let mut fork = vec![json!({"type": "session_meta", "payload": {"id": child,
            "forked_from_id": parent, "cwd": "/home/example/project", "cli_version": "0.150.0"}})];
        fork.extend(copied);
        let base = records / 2;
        for record in 0..records / 2 {
            if record % 10 == 0 {
                fork.push(turn(1_000 + family, record));
            }
            fork.push(content.clone());
            fork.push(count(base + record, base + record + 1));
        }
        write(&child, &fork);
    }
    let direct = "019f0000-0000-7000-8000-999999999999";
    let mut lines = vec![json!({"type": "session_meta", "payload": {"id": direct}})];
    for record in 0..records {
        lines.push(json!({"type": "token_usage_record", "payload": {"thread_id": direct,
            "response_id": format!("resp-{record}"),
            "usage": {"input_tokens": 100, "output_tokens": 10, "total_tokens": 110}}}));
    }
    write(direct, &lines);
    root
}

fn workers(count: usize) -> NonZeroUsize {
    NonZeroUsize::new(count).expect("positive workers")
}

/// Ingests the Codex home at `root` under `admission` on `count` workers, with the slot
/// size its discovery implies.
fn ingest_codex(
    root: &std::path::Path,
    admission: &MemoryAdmission,
    count: usize,
) -> Result<urollup_core::adapters::Ingested, AdapterError> {
    let discovery = discover(&codex_rollout::rollout_roots(&[root.to_owned()]));
    let slot = worker_slot(&[&discovery]);
    codex_rollout::ingest_discovery_with_admission(
        discovery,
        true,
        &Decoding { admission, workers: workers(count), slot },
    )
}

fn budget(bytes: u64) -> MemoryAdmission {
    MemoryAdmission::new(
        MemoryBudget::exact(bytes, format!("--max-ram {bytes}")),
        ProcessModel::DEFAULT,
    )
}

/// The smallest whole-process budget at which `run` succeeds, from a successful run's
/// largest heap term (`F + H × largest`): probed again at that budget until the slot
/// count it implies is the one the run charged, since the slots' charge depends on it.
fn threshold(slot: u64, run: impl Fn(&MemoryAdmission) -> bool) -> u64 {
    let slots = |bytes: u64| budget(bytes).worker_slots(slot, Some(workers(8)));
    let mut probe = u64::MAX / 4;
    loop {
        let admission = budget(probe);
        assert!(run(&admission), "the probe at {probe} succeeds");
        let threshold = ProcessModel::DEFAULT.whole_process(admission.largest());
        if slots(threshold) == slots(probe) {
            return threshold;
        }
        probe = threshold;
    }
}

fn slot_of(root: &std::path::Path) -> u64 {
    worker_slot(&[&discover(&codex_rollout::rollout_roots(&[root.to_owned()]))])
}

#[test]
fn a_codex_ingest_commits_what_it_retains_and_releases_its_decode() {
    use urollup_core::adapters::Retained;
    use urollup_core::ledger::admission::deep_size::DeepSize as _;
    use urollup_core::selection::Agent;
    let root = codex_families(2, 40, 0, true, true);
    for count in [1, 8] {
        let admission = MemoryAdmission::unlimited();
        admission.hold(&[(Hold::Discovery(Agent::Codex), 1_000)]).expect("the hold fits");
        let ingested = ingest_codex(root.path(), &admission, count).expect("the home ingests");
        // The forks copy their roots' history and wait for every rollout to decode.
        assert_eq!(ingested.relationships.len(), 2);
        assert!(ingested.ledger.coverage.copies > 0);
        let retained = Retained::of(Agent::Codex, &ingested);
        assert_eq!(retained.ledger, ingested.heap());
        assert_eq!(
            retained.query_reserve,
            ingested.ledger.requests.len() as u64 * model::QUERY_RESERVE
        );
        assert!(retained.index_reserve > 0);
        // The commit holds what the ledger retains, releases the discovery hold, and keeps
        // only the interns besides.
        assert_eq!(admission.held(Hold::Ledger(Agent::Codex)), retained.total());
        assert_eq!(admission.held(Hold::Discovery(Agent::Codex)), 0);
        assert_eq!(admission.held(Hold::Coordinating), 0);
        assert_eq!(
            admission.charged(),
            retained.total() + admission.charged_by(Component::Interns)
        );
        // Decode charged every component a record, its κ, its payloads, its limit rows and
        // its source and thread rows pay for.
        for component in [
            Component::Records,
            Component::Kappa,
            Component::Payloads,
            Component::Limits,
            Component::Sources,
        ] {
            assert!(admission.charged_by(component) > 0, "{component:?}");
        }
        assert!(admission.largest() > admission.charged());
    }
}

/// The budget at which a Codex ingest succeeds is exact and the same for one and eight
/// workers: one byte less refuses with the same error, and a budget that holds one worker
/// slot and the work order but not the first record refuses while reading, naming no
/// estimate.
#[test]
fn codex_refusal_is_exact_and_independent_of_the_worker_count() {
    use urollup_core::ledger::admission::{CapacityError, Phase};
    use urollup_core::selection::Agent;
    let root = codex_families(3, 120, 0, true, true);
    let slot = slot_of(root.path());
    // A first run interns the home's names, which later runs in this process share.
    let expected = ingest_codex(root.path(), &MemoryAdmission::unlimited(), 1).expect("ingests");
    let threshold = threshold(slot, |admission| ingest_codex(root.path(), admission, 1).is_ok());
    let mut refusals = Vec::new();
    for count in [1, 8] {
        for _ in 0..3 {
            let at = ingest_codex(root.path(), &budget(threshold), count)
                .expect("the threshold budget succeeds");
            assert_eq!(at.ledger, expected.ledger, "{count} workers");
            let error = ingest_codex(root.path(), &budget(threshold - 1), count)
                .expect_err("one byte less refuses");
            let AdapterError::Capacity(CapacityError::Memory { budget, .. }) = &error else {
                panic!("{count} workers: unexpected error {error}");
            };
            assert_eq!(*budget, threshold - 1);
            refusals.push(error.to_string());
        }
    }
    refusals.dedup();
    assert_eq!(refusals.len(), 1, "{refusals:#?}");

    // One slot and the work order, and not the first record.
    let sources = discover(&codex_rollout::rollout_roots(&[root.path().to_owned()])).sources.len();
    let floor = ProcessModel::DEFAULT.whole_process(slot + model::work_order(sources as u64));
    let error = ingest_codex(root.path(), &budget(floor), 1).expect_err("decode refuses");
    let AdapterError::Capacity(CapacityError::Memory { phase, estimate, .. }) = error else {
        panic!("unexpected error {error}");
    };
    assert_eq!((phase, estimate), (Phase::Decode(Agent::Codex), None));
}

/// Distinct rate-limit snapshots on every token count and a model per turn cost a Codex
/// history more than the same requests with one snapshot and one model: at the budget the
/// plain history needs, whose row-shell count admits every request, the dense one refuses.
/// Padding the skipped content changes nothing admission charges.
#[test]
fn high_cardinality_codex_histories_refuse_where_row_shells_admitted_them() {
    use urollup_core::ledger::admission::CapacityError;
    let plain = codex_families(4, 200, 0, false, false);
    let dense = codex_families(4, 200, 0, true, true);
    let padded = codex_families(4, 200, 4_096, true, true);
    let slot = slot_of(plain.path());
    let mut observations = 0;
    for root in [&plain, &dense, &padded] {
        let ingested =
            ingest_codex(root.path(), &MemoryAdmission::unlimited(), 1).expect("ingests");
        observations = ingested.ledger.coverage.observations;
    }
    let plain_threshold =
        threshold(slot, |admission| ingest_codex(plain.path(), admission, 1).is_ok());
    let dense_threshold =
        threshold(slot, |admission| ingest_codex(dense.path(), admission, 1).is_ok());
    assert!(dense_threshold > plain_threshold, "{dense_threshold} ≤ {plain_threshold}");
    // The row-shell count the plain history's budget implies admits every request.
    let rows = ObservationCapacity::from_byte_budget(plain_threshold, "row shells");
    assert!(rows.maximum() as u64 >= observations);
    let admission = budget(plain_threshold).with_row_ceiling(rows);
    let error = ingest_codex(dense.path(), &admission, 1).expect_err("the dense history refuses");
    assert!(matches!(error, AdapterError::Capacity(CapacityError::Memory { .. })), "{error}");
    // Skipped content of any size is never charged.
    let measure = |root: &tempfile::TempDir| {
        let admission = MemoryAdmission::unlimited();
        ingest_codex(root.path(), &admission, 1).expect("ingests");
        (admission.largest(), Component::ALL.map(|component| admission.charged_by(component)))
    };
    let (dense_largest, dense_charged) = measure(&dense);
    let (padded_largest, padded_charged) = measure(&padded);
    assert_eq!(padded_largest, dense_largest);
    assert_eq!(padded_charged, dense_charged);
}

/// The variable that makes [`threshold_child`] ingest once in a fresh process: the agent,
/// the home, the budget in bytes or `unlimited`, and the worker count, separated by tabs.
const CHILD: &str = "UROLLUP_ADMISSION_THRESHOLD_CHILD";

/// Ingests once and prints the outcome, when [`in_child`] runs this binary for it: in a
/// fresh process, no other ingest has interned a name, so every run charges the same new
/// interns, whichever worker interns each first. Without the variable it does nothing.
#[test]
fn threshold_child() {
    let Some(request) = std::env::var_os(CHILD) else { return };
    let request = request.into_string().expect("the request is UTF-8");
    let fields: Vec<&str> = request.split('\t').collect();
    let [agent, root, bytes, count] = fields[..] else { panic!("four fields: {request}") };
    let admission = match bytes {
        "unlimited" => MemoryAdmission::unlimited(),
        bytes => budget(bytes.parse().expect("a byte budget")),
    };
    let count = count.parse().expect("a worker count");
    let result = match agent {
        "codex" => ingest_codex(std::path::Path::new(root), &admission, count),
        agent => panic!("no adapter for {agent}"),
    };
    match result {
        Ok(_) => println!("outcome: ok largest {}", admission.largest()),
        Err(error) => println!("outcome: refused {error}"),
    }
}

/// Runs [`threshold_child`] in a fresh process of this binary and returns its outcome.
fn in_child(agent: &str, root: &std::path::Path, bytes: Option<u64>, count: usize) -> String {
    let bytes = bytes.map_or_else(|| "unlimited".to_owned(), |bytes| bytes.to_string());
    let request = format!("{agent}\t{}\t{bytes}\t{count}", root.display());
    let output = std::process::Command::new(std::env::current_exe().expect("this test binary"))
        .args(["--exact", "threshold_child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, request)
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(output.status.success(), "the child fails: {stdout}");
    stdout
        .lines()
        .find_map(|line| line.split_once("outcome: ").map(|(_, outcome)| outcome))
        .unwrap_or_else(|| panic!("the child reports no outcome: {stdout}"))
        .to_owned()
}

/// The largest heap term a successful child outcome reports.
fn largest_of(outcome: &str) -> u64 {
    outcome
        .strip_prefix("ok largest ")
        .and_then(|largest| largest.parse().ok())
        .unwrap_or_else(|| panic!("not a success: {outcome}"))
}

/// Admission's determinism in fresh processes (scalable-ingestion plan, "Admission
/// Tests", worker determinism): the threshold `T` computed from a successful run's
/// largest heap term succeeds and `T` − 1 refuses, with identical outcomes, on one and
/// eight workers across repeated runs, while new interns are charged cold in each.
#[test]
fn codex_thresholds_hold_in_fresh_processes_at_one_and_eight_workers() {
    let root = codex_families(3, 120, 0, true, true);
    let slot = slot_of(root.path());
    let slots = |bytes: Option<u64>| {
        bytes.map_or(8, |bytes| budget(bytes).worker_slots(slot, Some(workers(8))).get())
    };
    let mut probe = None;
    let threshold = loop {
        let largest = largest_of(&in_child("codex", root.path(), probe, 1));
        let threshold = ProcessModel::DEFAULT.whole_process(largest);
        if slots(Some(threshold)) == slots(probe) {
            break threshold;
        }
        probe = Some(threshold);
    };
    let mut at = Vec::new();
    let mut below = Vec::new();
    for count in [1, 8] {
        for _ in 0..3 {
            at.push(in_child("codex", root.path(), Some(threshold), count));
            below.push(in_child("codex", root.path(), Some(threshold - 1), count));
        }
    }
    at.dedup();
    below.dedup();
    assert_eq!(at.len(), 1, "{at:#?}");
    assert!(at[0].starts_with("ok "), "{at:#?}");
    assert_eq!(below.len(), 1, "{below:#?}");
    assert!(below[0].starts_with("refused estimated memory"), "{below:#?}");
}
