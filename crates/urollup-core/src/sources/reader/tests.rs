use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;

use super::{
    LogicalSource, RawRecord, ReadOptions, RecordDisposition, ScanHooks, SourceReadError,
    SourceSpec, read_source, read_source_with_hooks,
};
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::{
    CoverageFailure, ManifestEntry, PendingTail, Representation, SourceChange,
};

const SPEC: SourceSpec<'static> = SourceSpec {
    environment: "local",
    dialect: "test-dialect",
    locator: "project/session.jsonl",
    stable_locator: false,
};

fn options() -> ReadOptions {
    // No sleeping in tests: the hooks make the filesystem race deterministic.
    ReadOptions { absent_retry_delay: Duration::ZERO, ..ReadOptions::new() }
}

fn write(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn compress(contents: &[u8]) -> Vec<u8> {
    zstd::stream::encode_all(contents, 3).unwrap()
}

fn plain(path: &Path) -> LogicalSource {
    LogicalSource { plain: Some(path.to_owned()), compressed: None }
}

fn compressed(path: &Path) -> LogicalSource {
    LogicalSource { plain: None, compressed: Some(path.to_owned()) }
}

/// Reads a source, collecting every record's evidence and bytes.
fn scan(files: &LogicalSource) -> (ManifestEntry, Vec<(EvidenceRef, String)>) {
    scan_with(files, &options(), &mut super::NoHooks)
}

fn scan_with(
    files: &LogicalSource,
    options: &ReadOptions,
    hooks: &mut dyn ScanHooks,
) -> (ManifestEntry, Vec<(EvidenceRef, String)>) {
    let mut records = Vec::new();
    let entry = read_source_with_hooks(
        &SPEC,
        files,
        options,
        |record: &RawRecord<'_>| {
            records.push((
                record.evidence.clone(),
                String::from_utf8_lossy(record.bytes).into_owned(),
            ));
            if serde_json::from_slice::<serde_json::Value>(record.bytes).is_ok() {
                RecordDisposition::Decoded
            } else {
                RecordDisposition::Malformed
            }
        },
        hooks,
    )
    .unwrap();
    (entry, records)
}

/// Hooks that run one closure at one point of the scan.
struct At<F> {
    when: When,
    action: F,
    fired: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum When {
    Retry,
    AfterOpen,
    AfterScan,
}

impl<F: FnMut(&Path)> At<F> {
    fn new(when: When, action: F) -> Self {
        Self { when, action, fired: false }
    }

    fn fire(&mut self, when: When, path: &Path) {
        if self.when == when && !self.fired {
            self.fired = true;
            (self.action)(path);
        }
    }
}

impl<F: FnMut(&Path)> ScanHooks for At<F> {
    fn before_retry(&mut self, path: &Path, _attempt: u32) {
        self.fire(When::Retry, path);
    }

    fn after_open(&mut self, path: &Path) {
        self.fire(When::AfterOpen, path);
    }

    fn after_scan(&mut self, path: &Path) {
        self.fire(When::AfterScan, path);
    }
}

const THREE_RECORDS: &[u8] = b"{\"i\":1}\n{\"i\":2}\n{\"i\":3}\n";

#[test]
fn reads_complete_records_with_decoded_offsets_and_a_source_id() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);

    let (entry, records) = scan(&plain(&path));
    let source = entry.source.clone().unwrap();
    source.verify().unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].0, EvidenceRef { source: source.id.clone(), offset: 0, length: 7 });
    assert_eq!(records[1].0.offset, 8);
    assert_eq!(records[2].0.offset, 16);
    assert_eq!(records[2].1, "{\"i\":3}");
    assert_eq!(entry.counters.records, 3);
    assert_eq!(entry.counters.decoded, 3);
    assert_eq!(entry.cutoff.complete_through, 24);
    assert_eq!(entry.cutoff.pending_tail, None);
    assert_eq!(entry.representation, Representation::Plain);
    assert!(entry.changes.is_empty());
    assert!(entry.is_complete());
    assert_eq!(entry.file.path, path);
    #[cfg(unix)]
    assert!(entry.file.inode.is_some());
}

#[test]
fn an_unfinished_last_line_is_pending_not_corruption() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, b"{\"i\":1}\n{\"i\":2");

    let (entry, records) = scan(&plain(&path));
    assert_eq!(records.len(), 1);
    assert_eq!(entry.cutoff.pending_tail, Some(PendingTail { offset: 8, length: 6 }));
    assert_eq!(entry.cutoff.complete_through, 8);
    assert_eq!(entry.counters.malformed, 0);
    assert!(!entry.is_complete());

    // The next run reads the record once the writer finishes the line.
    write(&path, b"{\"i\":1}\n{\"i\":2}\n");
    let (finished, records) = scan(&plain(&path));
    assert_eq!(records.len(), 2);
    assert_eq!(finished.cutoff.pending_tail, None);
    assert_eq!(finished.source, entry.source, "appending keeps the source ID");
}

#[test]
fn interior_corruption_is_counted_and_the_scan_continues() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, b"{\"i\":1}\n{ truncated write\n{\"i\":3}\n\n   \n");

    let (entry, records) = scan(&plain(&path));
    assert_eq!(records.len(), 3);
    assert_eq!(entry.counters.malformed, 1);
    assert_eq!(entry.counters.decoded, 2);
    assert_eq!(entry.counters.blank_lines, 2);
    assert_eq!(entry.first_malformed.as_ref().unwrap().offset, 8);
    assert_eq!(entry.cutoff.pending_tail, None);
    assert!(!entry.is_complete());
}

#[test]
fn an_oversized_record_is_a_coverage_failure_never_a_silent_skip() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    let mut contents = Vec::from(b"{\"i\":1}\n" as &[u8]);
    contents.extend(std::iter::repeat_n(b'x', 5_000));
    contents.extend(b"\n{\"i\":3}\n");
    write(&path, &contents);

    let small = ReadOptions { max_record_bytes: 1_024, ..options() };
    let (entry, records) = scan_with(&plain(&path), &small, &mut super::NoHooks);
    assert_eq!(records.len(), 2, "the oversized record is not delivered");
    assert_eq!(records[1].0.offset, 5_009, "later records keep their true offsets");
    assert_eq!(entry.counters.oversized, 1);
    assert_eq!(entry.failures, vec![CoverageFailure::Oversized { offset: 8, length: 5_000 }]);
    assert!(!entry.is_complete());
}

#[test]
fn a_compressed_source_reads_like_its_plain_twin() {
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let compressed_path = root.path().join("session.jsonl.zst");
    write(&plain_path, THREE_RECORDS);
    write(&compressed_path, &compress(THREE_RECORDS));

    let (from_plain, plain_records) = scan(&plain(&plain_path));
    let (from_zstd, zstd_records) = scan(&compressed(&compressed_path));
    assert_eq!(from_zstd.representation, Representation::Zstd);
    assert_eq!(from_zstd.source, from_plain.source, "one logical source, one src- ID");
    assert_eq!(zstd_records, plain_records, "decoded offsets and bytes match");
    assert_eq!(from_zstd.cutoff.complete_through, from_plain.cutoff.complete_through);
    assert!(from_zstd.is_complete());
}

#[test]
fn a_multi_frame_compressed_source_reads_every_frame() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl.zst");
    let mut contents = compress(b"{\"i\":1}\n{\"i\":2}\n");
    contents.extend(compress(b"{\"i\":3}\n"));
    write(&path, &contents);

    let (entry, records) = scan(&compressed(&path));
    assert_eq!(records.len(), 3);
    assert_eq!(records[2].0.offset, 16);
    assert!(entry.failures.is_empty());
}

#[test]
fn a_compressed_stream_cut_short_is_an_incomplete_frame_not_corruption() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl.zst");
    let full = compress(THREE_RECORDS);
    write(&path, &full[..full.len() - 4]);

    let (entry, _records) = scan(&compressed(&path));
    assert!(
        matches!(entry.failures.as_slice(), [CoverageFailure::IncompleteCompressedFrame { .. }]),
        "{:?}",
        entry.failures
    );

    let corrupt_path = root.path().join("corrupt.jsonl.zst");
    let mut corrupt = full.clone();
    let middle = corrupt.len() / 2;
    corrupt[middle] ^= 0xff;
    write(&corrupt_path, &corrupt);
    // A zstd frame carries no content checksum unless its writer asked for one, so a
    // flipped byte inside a frame decodes to damaged text: interior corruption, counted,
    // rather than a decoder failure.
    let (corrupt_entry, _) = scan(&compressed(&corrupt_path));
    assert!(
        corrupt_entry.counters.malformed > 0 || !corrupt_entry.failures.is_empty(),
        "corruption inside a frame is reported one way or the other"
    );
}

#[test]
fn a_plain_file_and_its_compressed_twin_are_one_source() {
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let compressed_path = root.path().join("session.jsonl.zst");
    write(&plain_path, THREE_RECORDS);
    write(&compressed_path, &compress(THREE_RECORDS));

    let both = LogicalSource {
        plain: Some(plain_path.clone()),
        compressed: Some(compressed_path.clone()),
    };
    let (entry, records) = scan(&both);
    assert_eq!(entry.representation, Representation::Plain, "the plain file hides its twin");
    assert_eq!(entry.twin.as_ref().map(|twin| twin.path.clone()), Some(compressed_path.clone()));
    assert_eq!(records.len(), 3);
    assert!(entry.failures.is_empty());

    // A ".zst" beside an unrelated file of the same name is not a twin.
    write(&compressed_path, &compress(b"{\"other\":true}\n"));
    let (mismatched, _) = scan(&both);
    assert_eq!(mismatched.twin, None);
    assert_eq!(
        mismatched.failures,
        vec![CoverageFailure::TwinFingerprintMismatch {
            path: compressed_path,
            locator: SPEC.locator.to_owned(),
        }]
    );
}

#[test]
fn records_appended_after_the_cutoff_belong_to_the_next_snapshot() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);
    let appended = path.clone();
    let mut hooks = At::new(When::AfterOpen, move |_| {
        let mut file = fs::OpenOptions::new().append(true).open(&appended).unwrap();
        file.write_all(b"{\"i\":4}\n").unwrap();
    });

    let (entry, records) = scan_with(&plain(&path), &options(), &mut hooks);
    assert_eq!(records.len(), 3, "the scan stops at its cutoff");
    assert_eq!(entry.cutoff.complete_through, 24);
    assert_eq!(entry.changes, vec![SourceChange::GrewBeyondCutoff { observed_len: 32 }]);
    assert!(entry.is_complete(), "an append past the cutoff does not spoil the snapshot");
}

#[test]
fn truncation_during_a_scan_is_detected() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);
    let truncated = path.clone();
    let mut hooks = At::new(When::AfterOpen, move |_| {
        fs::OpenOptions::new().write(true).open(&truncated).unwrap().set_len(8).unwrap();
    });

    let (entry, records) = scan_with(&plain(&path), &options(), &mut hooks);
    assert_eq!(records.len(), 1);
    assert!(
        entry.changes.contains(&SourceChange::Truncated { snapshot_len: 24, observed_len: 8 }),
        "{:?}",
        entry.changes
    );
    assert!(!entry.is_complete());
}

#[test]
fn replacement_during_a_scan_is_detected() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);
    let replaced = path.clone();
    let staged = root.path().join("staged.jsonl");
    let mut hooks = At::new(When::AfterScan, move |_| {
        write(&staged, b"{\"replacement\":true}\n");
        fs::rename(&staged, &replaced).unwrap();
    });

    let (entry, records) = scan_with(&plain(&path), &options(), &mut hooks);
    assert_eq!(records.len(), 3, "the open file still holds the snapshot's records");
    #[cfg(unix)]
    assert_eq!(entry.changes, vec![SourceChange::Replaced]);
    #[cfg(not(unix))]
    assert!(!entry.changes.is_empty());
    assert!(!entry.is_complete());
}

#[test]
fn a_path_that_is_briefly_absent_is_retried_and_reported() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    let restored = path.clone();
    let mut hooks = At::new(When::Retry, move |_| write(&restored, THREE_RECORDS));

    let (entry, records) = scan_with(&plain(&path), &options(), &mut hooks);
    assert_eq!(records.len(), 3);
    assert_eq!(entry.changes, vec![SourceChange::BrieflyAbsent { attempts: 1 }]);
    assert!(entry.is_complete(), "a rewrite that restores the file loses no records");
}

#[test]
fn a_source_that_vanishes_during_a_scan_is_reported() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);
    let removed = path.clone();
    let mut hooks = At::new(When::AfterScan, move |_| fs::remove_file(&removed).unwrap());

    let (entry, records) = scan_with(&plain(&path), &options(), &mut hooks);
    assert_eq!(records.len(), 3);
    assert_eq!(entry.changes, vec![SourceChange::Vanished]);
    assert!(!entry.is_complete());
}

#[test]
fn rewriting_the_first_record_changes_the_source_id_and_is_reported() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);
    let rewritten = path.clone();
    let mut hooks =
        At::new(When::AfterScan, move |_| write(&rewritten, b"{\"i\":9}\n{\"i\":2}\n{\"i\":3}\n"));

    let (entry, _records) = scan_with(&plain(&path), &options(), &mut hooks);
    assert!(entry.changes.contains(&SourceChange::FirstRecordChanged), "{:?}", entry.changes);
    assert!(!entry.is_complete());

    let (rescanned, _) = scan(&plain(&path));
    assert_ne!(rescanned.source, entry.source, "a new first record is a new source");
    assert!(rescanned.changes.is_empty());
}

#[test]
fn unreadable_and_unnamed_sources_are_errors() {
    let root = TempDir::new().unwrap();
    let missing = root.path().join("gone.jsonl");
    let no_retry = ReadOptions { absent_retries: 0, ..options() };
    let error = read_source(&SPEC, &plain(&missing), &no_retry, |_| RecordDisposition::Skipped)
        .unwrap_err();
    assert!(matches!(error, SourceReadError::Open { .. }));
    assert!(format!("{error}").contains("gone.jsonl"));

    let empty = LogicalSource { plain: None, compressed: None };
    assert!(matches!(
        read_source(&SPEC, &empty, &options(), |_| RecordDisposition::Skipped),
        Err(SourceReadError::NoFile)
    ));

    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);
    let bad_spec = SourceSpec { environment: "Local Host", ..SPEC };
    let bad = read_source(&bad_spec, &plain(&path), &options(), |_| RecordDisposition::Skipped)
        .unwrap_err();
    assert!(matches!(bad, SourceReadError::Key { .. }));
}

#[test]
fn a_source_without_a_complete_record_has_no_id_and_reads_nothing() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, b"{\"unfinished\":true");
    let (entry, records) = scan(&plain(&path));
    assert!(records.is_empty());
    assert_eq!(entry.source, None);
    assert_eq!(entry.fingerprint, None);
    assert_eq!(entry.cutoff.pending_tail, Some(PendingTail { offset: 0, length: 18 }));

    let empty = root.path().join("empty.jsonl");
    write(&empty, b"");
    let (entry, records) = scan(&plain(&empty));
    assert!(records.is_empty());
    assert_eq!(
        entry.cutoff,
        crate::sources::manifest::Cutoff {
            complete_through: 0,
            pending_tail: None,
            captured_at: entry.cutoff.captured_at,
        }
    );
}

#[test]
fn every_record_of_a_windows_line_ending_file_is_complete() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, b"{\"i\":1}\r\n{\"i\":2}\r\n");
    let (entry, records) = scan(&plain(&path));
    assert_eq!(records.len(), 2);
    assert_eq!(entry.counters.decoded, 2, "a trailing carriage return is JSON whitespace");
    assert_eq!(records[1].0.offset, 9);
}

#[test]
fn the_manifest_records_one_cutoff_per_source() {
    let root = TempDir::new().unwrap();
    let mut manifest = crate::sources::manifest::SnapshotManifest::default();
    for name in ["a.jsonl", "b.jsonl"] {
        let path: PathBuf = root.path().join(name);
        write(&path, THREE_RECORDS);
        let (entry, _) = scan(&plain(&path));
        manifest.entries.push(entry);
    }
    assert_eq!(manifest.entries.len(), 2);
    assert!(manifest.cutoff_skew().is_some(), "per-source cutoffs give the skew across files");
}

#[test]
fn oversized_unterminated_tail_keeps_the_actual_snapshot_boundary() {
    let dir = TempDir::new().unwrap();
    let small = ReadOptions { max_record_bytes: 4, ..options() };
    for length in [4, 5, 20_000] {
        let mut bytes = Vec::from(b"{}\n" as &[u8]);
        bytes.extend(std::iter::repeat_n(b'x', length));
        let path = dir.path().join("input.jsonl");
        let compressed_path = dir.path().join("input.jsonl.zst");
        write(&path, &bytes);
        write(&compressed_path, &compress(&bytes));
        for files in [plain(&path), compressed(&compressed_path)] {
            let (entry, records) = scan_with(&files, &small, &mut super::NoHooks);
            assert_eq!(records.len(), 1);
            assert_eq!(entry.cutoff.complete_through, 3);
            assert_eq!(
                entry.cutoff.pending_tail,
                Some(PendingTail { offset: 3, length: length as u64 })
            );
            assert_eq!(entry.counters.oversized, u64::from(length > 4));
            if length > 4 {
                assert_eq!(
                    entry.failures,
                    [CoverageFailure::Oversized { offset: 3, length: length as u64 }]
                );
            }
        }
    }
}

#[test]
fn a_stopped_visitor_aborts_without_visiting_the_tail_or_returning_a_snapshot() {
    let dir = TempDir::new().unwrap();
    let bytes = b"{}\n{}\n{}\n";
    let plain_path = dir.path().join("input.jsonl");
    let compressed_path = dir.path().join("input.jsonl.zst");
    write(&plain_path, bytes);
    write(&compressed_path, &compress(bytes));
    for files in [plain(&plain_path), compressed(&compressed_path)] {
        let mut visited = 0;
        let result = read_source(&SPEC, &files, &options(), |_| {
            visited += 1;
            if visited == 2 { RecordDisposition::Stop } else { RecordDisposition::Decoded }
        });
        assert!(matches!(result, Err(SourceReadError::VisitorStopped)));
        assert_eq!(visited, 2);
    }
}
