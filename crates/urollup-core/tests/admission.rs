//! Configured admission limits apply across worker results before reconciliation.

use std::fs;
use std::mem::size_of;
use std::num::NonZeroUsize;

use serde_json::json;
use urollup_core::adapters::{AdapterError, claude_project, codex_rollout};
use urollup_core::ledger::capacity::ObservationCapacity;
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
