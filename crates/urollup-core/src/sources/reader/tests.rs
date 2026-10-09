use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;

use super::{
    LogicalSource, RawRecord, ReadOptions, RecordDisposition, ScanHooks, SourceReadError,
    SourceSpec, peek, read_source, read_source_with_hooks,
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

fn gzip(contents: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(contents).unwrap();
    encoder.finish().unwrap()
}

fn plain(path: &Path) -> LogicalSource {
    LogicalSource::single(path.to_owned(), Representation::Plain)
}

fn compressed(path: &Path) -> LogicalSource {
    LogicalSource::single(path.to_owned(), Representation::Zstd)
}

fn gzipped(path: &Path) -> LogicalSource {
    LogicalSource::single(path.to_owned(), Representation::Gzip)
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
            records.push((*record.evidence, String::from_utf8_lossy(record.bytes).into_owned()));
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
    assert_eq!(records[0].0, EvidenceRef { source: 0, offset: 0, length: 7 });
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
        zstd: Some(compressed_path.clone()),
        gzip: None,
    };
    let (entry, records) = scan(&both);
    assert_eq!(entry.representation, Representation::Plain, "the plain file hides its twin");
    assert_eq!(twin_paths(&entry), vec![compressed_path.clone()]);
    assert_eq!(records.len(), 3);
    assert!(entry.failures.is_empty());

    // A ".zst" beside an unrelated file of the same name is not a twin.
    write(&compressed_path, &compress(b"{\"other\":true}\n"));
    let (mismatched, _) = scan(&both);
    assert!(mismatched.twins.is_empty());
    assert_eq!(
        mismatched.failures,
        vec![CoverageFailure::TwinFingerprintMismatch {
            path: compressed_path,
            locator: SPEC.locator.to_owned(),
        }]
    );
}

fn twin_paths(entry: &ManifestEntry) -> Vec<PathBuf> {
    entry.twins.iter().map(|twin| twin.path.clone()).collect()
}

#[test]
fn a_gzip_source_reads_like_its_plain_twin_across_members() {
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let gzip_path = root.path().join("session.jsonl.gz");
    write(&plain_path, THREE_RECORDS);
    // Two members, as `cat a.gz b.gz` or an appending compressor writes.
    let mut members = gzip(b"{\"i\":1}\n{\"i\":2}\n");
    members.extend(gzip(b"{\"i\":3}\n"));
    write(&gzip_path, &members);

    let (from_plain, plain_records) = scan(&plain(&plain_path));
    let (from_gzip, gzip_records) = scan(&gzipped(&gzip_path));
    assert_eq!(from_gzip.representation, Representation::Gzip);
    assert_eq!(from_gzip.source, from_plain.source, "one logical source, one src- ID");
    assert_eq!(gzip_records, plain_records, "decoded offsets and bytes match");
    assert_eq!(from_gzip.cutoff.complete_through, from_plain.cutoff.complete_through);
    assert!(from_gzip.is_complete(), "{:?}", from_gzip.failures);
}

#[test]
fn a_damaged_gzip_stream_is_reported_rather_than_read_as_complete() {
    let root = TempDir::new().unwrap();
    let full = gzip(THREE_RECORDS);

    let cut = root.path().join("cut.jsonl.gz");
    write(&cut, &full[..full.len() - 4]);
    let (cut_entry, _) = scan(&gzipped(&cut));
    assert!(
        matches!(
            cut_entry.failures.as_slice(),
            [CoverageFailure::IncompleteCompressedFrame { .. }]
        ),
        "{:?}",
        cut_entry.failures
    );
    assert!(!cut_entry.is_complete());

    // The CRC-32 at the end of the member catches a flipped byte in the stored text.
    let flipped = root.path().join("flipped.jsonl.gz");
    let mut damaged = full.clone();
    let checksum_offset = damaged.len() - 8;
    damaged[checksum_offset] ^= 0xff;
    write(&flipped, &damaged);
    let (flipped_entry, _) = scan(&gzipped(&flipped));
    assert!(
        matches!(
            flipped_entry.failures.as_slice(),
            [CoverageFailure::CorruptCompressedData { .. }]
        ),
        "{:?}",
        flipped_entry.failures
    );

    let garbage = root.path().join("garbage.jsonl.gz");
    write(&garbage, b"not gzip at all\n");
    let (garbage_entry, records) = scan(&gzipped(&garbage));
    assert!(records.is_empty());
    assert!(
        matches!(
            garbage_entry.failures.as_slice(),
            [CoverageFailure::CorruptCompressedData { .. }]
        ),
        "{:?}",
        garbage_entry.failures
    );
}

#[test]
fn compressed_empty_streams_are_empty_sources_but_zero_byte_files_are_incomplete() {
    let root = TempDir::new().unwrap();
    for (files, encoded) in [
        (compressed(&root.path().join("empty.jsonl.zst")), compress(b"")),
        (gzipped(&root.path().join("empty.jsonl.gz")), gzip(b"")),
    ] {
        let (path, _) = files.primary().unwrap();
        write(path, &encoded);
        let (entry, records) = scan(&files);
        assert!(records.is_empty());
        assert_eq!(entry.source, None);
        assert!(entry.is_complete(), "{path:?}: {:?}", entry.failures);

        // A compressor interrupted before its first byte leaves no valid stream.
        write(path, b"");
        let (interrupted, _) = scan(&files);
        assert_eq!(
            interrupted.failures,
            vec![CoverageFailure::IncompleteCompressedFrame { decoded_offset: 0 }],
            "{path:?}"
        );
    }
}

#[test]
fn every_twin_of_a_source_is_verified_in_preference_order() {
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let zstd_path = root.path().join("session.jsonl.zst");
    let gzip_path = root.path().join("session.jsonl.gz");
    write(&plain_path, THREE_RECORDS);
    write(&zstd_path, &compress(THREE_RECORDS));
    write(&gzip_path, &gzip(THREE_RECORDS));
    let all = LogicalSource {
        plain: Some(plain_path.clone()),
        zstd: Some(zstd_path.clone()),
        gzip: Some(gzip_path.clone()),
    };

    let (entry, records) = scan(&all);
    assert_eq!(records.len(), 3, "one representation is read, not three");
    assert_eq!(entry.representation, Representation::Plain);
    assert_eq!(twin_paths(&entry), vec![zstd_path.clone(), gzip_path.clone()]);
    assert!(entry.is_complete());

    // Without the plain file, zstd is read and gzip is its twin.
    let compressed_pair = LogicalSource { plain: None, ..all.clone() };
    write(&gzip_path, &gzip(b"{\"other\":true}\n"));
    let (pair, _) = scan(&compressed_pair);
    assert_eq!(pair.representation, Representation::Zstd);
    assert!(pair.twins.is_empty());
    assert_eq!(
        pair.failures,
        vec![CoverageFailure::TwinFingerprintMismatch {
            path: gzip_path,
            locator: SPEC.locator.to_owned(),
        }]
    );
}

#[test]
fn a_twin_whose_stream_ends_before_its_first_record_is_neither_verified_nor_a_loss() {
    // gzip and zstd write their output under its final name and remove the input only when
    // they finish, so a twin still being written sits beside the complete plain file. Every
    // proper prefix of a valid stream ends early, as these do.
    let gzipped = gzip(THREE_RECORDS);
    let zstd = compress(THREE_RECORDS);
    for (name, unfinished) in [
        ("session.jsonl.zst", Vec::new()),
        ("session.jsonl.zst", zstd[..6].to_vec()),
        ("session.jsonl.gz", gzipped[..12].to_vec()),
    ] {
        let root = TempDir::new().unwrap();
        let plain_path = root.path().join("session.jsonl");
        let twin_path = root.path().join(name);
        write(&plain_path, THREE_RECORDS);
        write(&twin_path, &unfinished);
        let mut files = plain(&plain_path);
        files.insert(twin_path, Representation::of_path(Path::new(name)).unwrap());

        let (entry, records) = scan(&files);
        assert_eq!(records.len(), 3, "{name} {unfinished:?}");
        assert!(entry.twins.is_empty(), "{name}: a twin with no first record is not verified");
        assert!(
            entry.failures.is_empty(),
            "{name}: nor is it another source: {:?}",
            entry.failures
        );
        assert!(entry.is_complete(), "{name}");
    }
}

#[test]
fn a_twin_whose_first_record_cannot_be_decoded_is_a_loss() {
    // Bytes that are not a compressed stream are not a compressor's unfinished output: the
    // twin may hold records the file read lacks, and nothing proves it does not.
    for name in ["session.jsonl.gz", "session.jsonl.zst"] {
        let root = TempDir::new().unwrap();
        let plain_path = root.path().join("session.jsonl");
        let twin_path = root.path().join(name);
        write(&plain_path, THREE_RECORDS);
        write(&twin_path, b"not a compressed stream at all\n");
        let mut files = plain(&plain_path);
        files.insert(twin_path.clone(), Representation::of_path(Path::new(name)).unwrap());

        let (entry, records) = scan(&files);
        assert_eq!(records.len(), 3, "{name}");
        assert!(entry.twins.is_empty(), "{name}");
        assert_eq!(
            entry.failures,
            vec![CoverageFailure::UnreadableTwin {
                path: twin_path,
                locator: SPEC.locator.to_owned(),
            }],
            "{name}"
        );
        assert_eq!(entry.losses(), ["unreadable-twin"], "{name}");
    }

    // With an empty primary, the damaged twin is the only file that might hold records.
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let gzip_path = root.path().join("session.jsonl.gz");
    write(&plain_path, b"");
    write(&gzip_path, b"not gzip at all\n");
    let files =
        LogicalSource { plain: Some(plain_path), zstd: None, gzip: Some(gzip_path.clone()) };
    let (entry, records) = scan(&files);
    assert!(records.is_empty());
    assert_eq!(
        entry.failures,
        vec![CoverageFailure::UnreadableTwin { path: gzip_path, locator: SPEC.locator.to_owned() }]
    );
    assert!(!entry.is_complete());
}

/// A complete line longer than `limit`, then `rest`.
fn oversized_then(limit: usize, rest: &[u8]) -> Vec<u8> {
    let mut contents = Vec::from(b"{\"pad\":\"" as &[u8]);
    contents.extend(std::iter::repeat_n(b'x', limit));
    contents.extend(b"\"}\n");
    contents.extend(rest);
    contents
}

#[test]
fn a_first_record_over_the_bound_is_skipped_by_twin_checks_as_by_the_scan() {
    let small = ReadOptions { max_record_bytes: 1_024, ..options() };

    // Read alone, the oversized line is a coverage failure and the next record fixes the
    // source ID; re-reading the first record after the scan agrees.
    let root = TempDir::new().unwrap();
    let gzip_path = root.path().join("session.jsonl.gz");
    write(&gzip_path, &gzip(&oversized_then(1_024, THREE_RECORDS)));
    let (alone, records) = scan_with(&gzipped(&gzip_path), &small, &mut super::NoHooks);
    assert_eq!(records.len(), 3);
    assert!(matches!(alone.failures.as_slice(), [CoverageFailure::Oversized { .. }]));
    assert!(alone.changes.is_empty(), "the first record did not change: {:?}", alone.changes);

    // An empty plain file gives way to that twin, whose records are read and whose
    // oversized line is still a loss.
    let plain_path = root.path().join("session.jsonl");
    write(&plain_path, b"");
    let files =
        LogicalSource { plain: Some(plain_path.clone()), zstd: None, gzip: Some(gzip_path) };
    let (entry, records) = scan_with(&files, &small, &mut super::NoHooks);
    assert_eq!(records.len(), 3, "the twin's records are read");
    assert_eq!(entry.representation, Representation::Gzip);
    assert_eq!(entry.losses(), ["oversized-record"]);

    // Beside a complete plain file, a twin of another source whose first line is
    // oversized is still a different, unread source.
    let other = root.path().join("other.jsonl.gz");
    write(&other, &gzip(&oversized_then(1_024, b"{\"other\":1}\n")));
    write(&plain_path, THREE_RECORDS);
    let files = LogicalSource { plain: Some(plain_path), zstd: None, gzip: Some(other.clone()) };
    let (entry, records) = scan_with(&files, &small, &mut super::NoHooks);
    assert_eq!(records.len(), 3);
    assert_eq!(
        entry.failures,
        vec![CoverageFailure::TwinFingerprintMismatch {
            path: other,
            locator: SPEC.locator.to_owned(),
        }]
    );

    // A twin holding only an oversized record has no first record to compare.
    let only = root.path().join("only.jsonl.gz");
    write(&only, &gzip(&oversized_then(1_024, b"")));
    let files = LogicalSource {
        plain: Some(root.path().join("session.jsonl")),
        zstd: None,
        gzip: Some(only.clone()),
    };
    let (entry, _) = scan_with(&files, &small, &mut super::NoHooks);
    assert_eq!(
        entry.failures,
        vec![CoverageFailure::UnreadableTwin { path: only, locator: SPEC.locator.to_owned() }]
    );
}

#[test]
fn a_damaged_primary_read_through_its_twin_keeps_its_own_loss() {
    let root = TempDir::new().unwrap();
    let zstd_path = root.path().join("session.jsonl.zst");
    let gzip_path = root.path().join("session.jsonl.gz");
    write(&zstd_path, b"not zstd at all\n");
    write(&gzip_path, &gzip(THREE_RECORDS));
    let files = LogicalSource { plain: None, zstd: Some(zstd_path.clone()), gzip: Some(gzip_path) };

    let (entry, records) = scan(&files);
    assert_eq!(records.len(), 3, "the twin's records are read");
    assert_eq!(entry.representation, Representation::Gzip);
    assert_eq!(
        entry.changes,
        vec![SourceChange::ReadFromOtherRepresentation {
            primary: zstd_path.clone(),
            representation: Representation::Gzip,
        }]
    );
    assert_eq!(entry.losses(), ["corrupt-compressed-data"], "{:?}", entry.failures);

    // A primary that only ends early, as while `zstd` converts the gzip file, lost nothing.
    write(&zstd_path, &compress(THREE_RECORDS)[..6]);
    let (entry, records) = scan(&files);
    assert_eq!(records.len(), 3);
    assert!(entry.losses().is_empty(), "{:?}", entry.failures);
    assert!(entry.is_complete());
}

#[test]
fn a_primary_that_reappears_empty_keeps_its_retry_record_when_a_twin_is_read() {
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let gzip_path = root.path().join("session.jsonl.gz");
    write(&gzip_path, &gzip(THREE_RECORDS));
    let files = LogicalSource {
        plain: Some(plain_path.clone()),
        zstd: None,
        gzip: Some(gzip_path.clone()),
    };
    // A decompressor creates the plain file while it is being retried.
    let created = plain_path.clone();
    let mut hooks = At::new(When::Retry, move |_| write(&created, b""));

    let (entry, records) = scan_with(&files, &options(), &mut hooks);
    assert_eq!(records.len(), 3);
    assert_eq!(
        entry.changes,
        vec![
            SourceChange::BrieflyAbsent { attempts: 1 },
            SourceChange::ReadFromOtherRepresentation {
                primary: plain_path,
                representation: Representation::Gzip,
            },
        ]
    );
    assert!(entry.is_complete());
}

#[test]
fn a_primary_without_a_complete_record_is_read_from_a_twin_that_has_one() {
    // `gunzip -k`, `zstd -d` or a Codex resume creates the plain file before it writes the
    // first line, so the plain file holds nothing its compressed twin lacks.
    for unfinished in [&b""[..], b"{\"i\":1"] {
        let root = TempDir::new().unwrap();
        let plain_path = root.path().join("session.jsonl");
        let gzip_path = root.path().join("session.jsonl.gz");
        write(&plain_path, unfinished);
        write(&gzip_path, &gzip(THREE_RECORDS));
        let files = LogicalSource {
            plain: Some(plain_path.clone()),
            zstd: None,
            gzip: Some(gzip_path.clone()),
        };

        let (entry, records) = scan(&files);
        let label = String::from_utf8_lossy(unfinished);
        assert_eq!(records.len(), 3, "{label}: the twin's records are read");
        assert_eq!(entry.file.path, gzip_path, "{label}");
        assert_eq!(entry.representation, Representation::Gzip, "{label}");
        assert!(entry.twins.is_empty(), "{label}: the unread primary is not a verified twin");
        assert_eq!(
            entry.changes,
            vec![SourceChange::ReadFromOtherRepresentation {
                primary: plain_path,
                representation: Representation::Gzip,
            }],
            "{label}"
        );
        assert!(entry.is_complete(), "{label}: {:?}", entry.failures);
    }
}

#[test]
fn a_source_compressed_after_discovery_is_read_from_its_new_file() {
    for (suffix, encode) in [
        (".jsonl.zst", compress as fn(&[u8]) -> Vec<u8>),
        (".jsonl.gz", gzip as fn(&[u8]) -> Vec<u8>),
    ] {
        let root = TempDir::new().unwrap();
        let discovered = root.path().join("session.jsonl");
        let replacement = root.path().join(format!("session{suffix}"));
        // `zstd --rm` or `gzip` replaces the file while the plain path is being retried.
        let written = replacement.clone();
        let mut hooks = At::new(When::Retry, move |_| write(&written, &encode(THREE_RECORDS)));

        let (entry, records) = scan_with(&plain(&discovered), &options(), &mut hooks);
        assert_eq!(records.len(), 3, "{suffix}");
        assert_eq!(entry.file.path, replacement);
        assert_eq!(
            entry.changes,
            vec![SourceChange::ReadFromOtherRepresentation {
                primary: discovered,
                representation: entry.representation,
            }],
            "{suffix}: the manifest says the discovered file was not read"
        );
        assert!(entry.is_complete(), "{suffix}: {:?} {:?}", entry.failures, entry.changes);
    }
}

#[test]
fn a_discovered_twin_is_read_when_the_primary_is_gone() {
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let gzip_path = root.path().join("session.jsonl.gz");
    write(&gzip_path, &gzip(THREE_RECORDS));
    let files = LogicalSource {
        plain: Some(plain_path.clone()),
        zstd: None,
        gzip: Some(gzip_path.clone()),
    };

    let (entry, records) = scan(&files);
    assert_eq!(records.len(), 3);
    assert_eq!(entry.representation, Representation::Gzip);
    assert_eq!(entry.file.path, gzip_path);
    assert!(entry.twins.is_empty());
    assert_eq!(
        entry.changes,
        vec![SourceChange::ReadFromOtherRepresentation {
            primary: plain_path,
            representation: Representation::Gzip,
        }]
    );
    assert!(entry.is_complete());
}

/// Discovery records only regular files and links that stay inside the declared roots, so
/// a file it never saw is opened only when it is a regular file.
#[cfg(unix)]
#[test]
fn an_undiscovered_file_that_is_not_a_regular_file_is_never_read() {
    let outside = TempDir::new().unwrap();
    let outside_source = outside.path().join("elsewhere.jsonl.gz");
    write(&outside_source, &gzip(THREE_RECORDS));
    let root = TempDir::new().unwrap();
    let other_source = root.path().join("b/other.jsonl.gz");
    write(&other_source, &gzip(THREE_RECORDS));

    for (name, target) in [("outside", &outside_source), ("other-source", &other_source)] {
        let discovered = root.path().join(format!("a/{name}.jsonl"));
        fs::create_dir_all(discovered.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, root.path().join(format!("a/{name}.jsonl.gz"))).unwrap();
        let (entry, records) = scan(&plain(&discovered));
        assert!(records.is_empty(), "{name}: a link discovery did not follow is not read");
        assert_eq!(entry.changes, vec![SourceChange::Vanished], "{name}");
        assert!(!entry.is_complete(), "{name}");
    }

    // Opening a FIFO blocks until a writer appears, which would hang the worker.
    let discovered = root.path().join("fifo.jsonl");
    let fifo = root.path().join("fifo.jsonl.gz");
    let made = std::process::Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success(), "mkfifo creates the FIFO");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || sender.send(scan(&plain(&discovered))).unwrap());
    let (entry, records) = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("a FIFO beside a vanished source does not block the read");
    assert!(records.is_empty());
    assert_eq!(entry.changes, vec![SourceChange::Vanished]);
}

#[test]
fn a_source_deleted_after_discovery_is_an_empty_vanished_snapshot() {
    let root = TempDir::new().unwrap();
    let missing = root.path().join("expired.jsonl.zst");

    let (entry, records) = scan(&compressed(&missing));
    assert!(records.is_empty());
    assert_eq!(entry.source, None);
    assert_eq!(entry.file.path, missing);
    assert_eq!(entry.changes, vec![SourceChange::Vanished]);
    assert!(!entry.is_complete(), "lost records are never reported as complete");
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
fn a_source_removed_after_a_complete_scan_is_reported_but_not_a_loss() {
    // The open file supplied every record of its extent, and it is unchanged, so a
    // compressor finishing (`zstd --rm`, `gzip`) or an agent expiring the file after the
    // scan loses nothing.
    for compress_first in [false, true] {
        let root = TempDir::new().unwrap();
        let path = root.path().join("session.jsonl");
        write(&path, THREE_RECORDS);
        let removed = path.clone();
        let mut hooks = At::new(When::AfterScan, move |_| {
            if compress_first {
                write(&removed.with_extension("jsonl.gz"), &gzip(THREE_RECORDS));
            }
            fs::remove_file(&removed).unwrap();
        });

        let (entry, records) = scan_with(&plain(&path), &options(), &mut hooks);
        assert_eq!(records.len(), 3, "compressed first: {compress_first}");
        assert_eq!(entry.file.path, path);
        assert_eq!(entry.changes, vec![SourceChange::RemovedAfterScan], "{compress_first}");
        assert!(entry.losses().is_empty(), "{compress_first}");
        assert!(entry.is_complete(), "{compress_first}");
    }
}

#[cfg(unix)]
#[test]
fn a_change_to_the_open_file_is_still_a_loss_when_its_path_is_gone() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    write(&path, THREE_RECORDS);
    let changed = path.clone();
    let mut hooks = At::new(When::AfterOpen, move |_| {
        fs::OpenOptions::new().write(true).open(&changed).unwrap().set_len(8).unwrap();
        fs::remove_file(&changed).unwrap();
    });

    let (entry, records) = scan_with(&plain(&path), &options(), &mut hooks);
    assert_eq!(records.len(), 1);
    assert_eq!(
        entry.changes,
        vec![
            SourceChange::Truncated { snapshot_len: 24, observed_len: 8 },
            SourceChange::RemovedAfterScan,
        ]
    );
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let closed = root.path().join("closed.jsonl");
        write(&closed, THREE_RECORDS);
        fs::set_permissions(&closed, fs::Permissions::from_mode(0o000)).unwrap();
        // A process that bypasses mode bits, such as root, opens the file anyway, so the
        // error cannot occur there; decide that up front rather than from the result.
        let bypasses_modes = fs::File::open(&closed).is_ok();
        let result =
            read_source(&SPEC, &plain(&closed), &options(), |_| RecordDisposition::Skipped);
        fs::set_permissions(&closed, fs::Permissions::from_mode(0o644)).unwrap();
        if bypasses_modes {
            eprintln!("skipped the open error: this process can open a mode-000 file");
        } else {
            let error = result.expect_err("an unreadable file is an open error, not a snapshot");
            assert!(matches!(error, SourceReadError::Open { .. }), "{error}");
            assert!(format!("{error}").contains("closed.jsonl"));
        }
    }

    let empty = LogicalSource::default();
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

#[test]
fn spare_capacity_after_a_long_line_is_released() {
    let mut buffer = Vec::with_capacity(2 * 1024 * 1024);
    buffer.resize(8, 1);
    super::shrink_line_buffer(&mut buffer);
    assert!(buffer.capacity() <= ReadOptions::RETAINED_LINE_CAPACITY);
    assert_eq!(buffer, [1; 8]);
}

#[test]
fn a_long_line_does_not_prevent_reading_the_next_record() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("session.jsonl");
    let long = format!("{{\"n\":1,\"pad\":\"{}\"}}", "x".repeat(300_000));
    write(&path, format!("{long}\n{{\"n\":2}}\n").as_bytes());
    let (entry, records) = scan(&plain(&path));
    assert_eq!(records.len(), 2);
    assert_eq!(entry.counters.decoded, 2);
    assert_eq!(records[1].1, "{\"n\":2}");
}

#[test]
fn a_peek_reads_the_first_file_of_the_source_that_the_look_accepts() {
    let root = TempDir::new().unwrap();
    let plain_path = root.path().join("session.jsonl");
    let gzip_path = root.path().join("session.jsonl.gz");
    // The plain file is still being written; its gzip twin is complete.
    write(&plain_path, b"{\"i\":1");
    write(&gzip_path, &gzip(THREE_RECORDS));
    let files = LogicalSource { plain: Some(plain_path), zstd: None, gzip: Some(gzip_path) };
    let first_line = |reader: &mut dyn std::io::BufRead| {
        let mut line = Vec::new();
        reader.read_until(b'\n', &mut line).ok()?;
        line.ends_with(b"\n").then_some(line)
    };

    assert_eq!(peek(&files, first_line), Some(b"{\"i\":1}\n".to_vec()));
    assert_eq!(peek(&files, |_| None::<()>), None, "a look that accepts nothing finds nothing");
    assert_eq!(peek(&LogicalSource::default(), first_line), None);
}
