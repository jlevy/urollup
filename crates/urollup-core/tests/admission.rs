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
