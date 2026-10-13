//! The streaming complete-record reader and its snapshot boundary (design §2.2).
//!
//! [`read_source`] freezes one source: it opens the file, takes its identity and length as
//! the snapshot extent, and streams complete records within that extent to a visitor,
//! stopping at the cutoff even if the file keeps growing. It reports rather than hides
//! everything that can lose data:
//!
//! - **Pending tails versus interior corruption.** Bytes after the last line terminator
//!   are a [`PendingTail`], not a record: an active writer's half-written line is read by
//!   the next run. A newline-terminated record that does not parse is interior corruption,
//!   counted in [`RecordCounters::malformed`] with the first one's evidence kept.
//! - **Changes during the scan.** The file's identity, length and modification time are
//!   compared before and after, and the first record's fingerprint is re-read, so
//!   replacement, truncation and in-place mutation each become a [`SourceChange`]. A path
//!   that is briefly absent while another tool rewrites it is retried and reported as
//!   [`SourceChange::BrieflyAbsent`] rather than counted as a lost source. A path gone
//!   after the scan is [`SourceChange::RemovedAfterScan`], which loses nothing: the open
//!   file supplied the snapshot, so it is checked instead of the path. An in-place
//!   mutation that keeps both the length and the modification time is the one case this
//!   check misses.
//! - **Files that disappear after discovery.** A compressor such as `zstd --rm` or
//!   `gzip` replaces a `.jsonl` file with its compressed form, and agents delete expired
//!   transcripts. When the discovered file is gone, the next representation of the same
//!   source is read instead and recorded as [`SourceChange::ReadFromOtherRepresentation`].
//!   One written after discovery is read only when it is a regular file, the rule
//!   discovery applies, so a link or FIFO beside the source is never opened. When none
//!   remains, the source is an empty snapshot with [`SourceChange::Vanished`], not an
//!   error.
//! - **Oversized records.** A record over [`ReadOptions::max_record_bytes`] is never
//!   silently skipped: it is counted and recorded as [`CoverageFailure::Oversized`], and
//!   the scan continues at the next record without buffering it.
//! - **Memory admission.** Every read takes a [`MemoryAdmission`]. A line buffer starts at
//!   [`ReadOptions::RETAINED_LINE_CAPACITY`] and grows only by doubling, up to
//!   [`ReadOptions::max_record_bytes`]; past the worker slot's 4 MiB it takes the
//!   large-record permit for its need (`4 × C′`) before it grows, and keeps it until the
//!   scan shrinks the buffer once the visitor returns. A zstd frame whose window passes
//!   the slot's 8 MiB takes the permit too (`frames`). A read that waits for the permit
//!   while admission refuses the invocation stops with [`SourceReadError::Stopped`].
//! - **Compression.** `.jsonl`, `.jsonl.zst` and `.jsonl.gz` decode to the same bytes, so
//!   evidence offsets are decoded offsets and a `.jsonl` file and its compressed twins are
//!   one logical source with one `src-` ID. Each other file of the source is a verified
//!   twin when its first record matches the file read, a different, unread source when it
//!   differs, and [`CoverageFailure::UnreadableTwin`] when it cannot be decoded or holds
//!   only an oversized record. Only a file whose stream ends before its first record, as
//!   every prefix of a valid stream and so a compressor's unfinished output does, is
//!   neither. When the file read has no complete record but another file of the source
//!   has one, that file is read instead, and the file read keeps any failure other than
//!   an early end. First records are found as the scan finds them, passing over blank and
//!   oversized lines. [`decode`] is the one place a representation maps to a decoder, for
//!   the scan, its first-record checks and [`peek`] alike. A compressed stream that ends
//!   inside a frame or member is
//!   [`CoverageFailure::IncompleteCompressedFrame`], distinct from
//!   [`CoverageFailure::CorruptCompressedData`]. Only structural damage fails a zstd
//!   decoder: a zstd frame carries a content checksum only when its writer asked for one,
//!   so a flipped byte inside a frame usually decodes to damaged text and is counted as
//!   interior corruption instead. Every gzip member ends with a CRC-32, so the same flip
//!   in a `.jsonl.gz` is also corrupt compressed data, found when the member ends.
//!
//! The `src-` ID is derived from the first complete record's [`Fingerprint`], so records
//! appended later keep the ID, and a rewrite of the first record produces a new one.

use std::fs::{File, Metadata};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::ledger::admission::model::{SLOT_LINE_CAPACITY, line_need, next_line_capacity};
use crate::ledger::admission::{LargeRecord, MemoryAdmission};
use crate::ledger::identity::{IdentityError, KeyComponent, StoredIdentity};
use crate::ledger::scope::{KeyScopeError, KeySpec};
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::{
    CoverageFailure, Cutoff, FileIdentity, Fingerprint, ManifestEntry, PendingTail, RecordCounters,
    Representation, SOURCE_ROOT_RELATIVE, SOURCE_STABLE_LOCATOR, SourceChange,
};

/// Limits and retry policy for one scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadOptions {
    /// The largest record the reader buffers. A longer record is a coverage failure.
    pub max_record_bytes: usize,
    /// How many times to re-check a path that is absent when it should not be.
    pub absent_retries: u32,
    /// How long to wait between those checks.
    pub absent_retry_delay: Duration,
}

impl ReadOptions {
    /// 64 MiB: larger than any record these dialects write, small enough that one
    /// worker's buffer stays bounded.
    pub const DEFAULT_MAX_RECORD_BYTES: usize = 64 * 1024 * 1024;

    /// Capacity kept after a long line so the next short lines do not reallocate, without
    /// retaining a 64 MiB buffer for the rest of the file.
    pub const RETAINED_LINE_CAPACITY: usize = 256 * 1024;

    /// The default limits: 64 MiB records, three re-checks 20 ms apart.
    pub fn new() -> Self {
        Self {
            max_record_bytes: Self::DEFAULT_MAX_RECORD_BYTES,
            absent_retries: 3,
            absent_retry_delay: Duration::from_millis(20),
        }
    }
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// What the visitor did with a record, counted per source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordDisposition {
    /// Abort the scan without returning a partial manifest.
    Stop,
    /// The record was decoded and used.
    Decoded,
    /// The record was deliberately not decoded, including a prefilter miss.
    Skipped,
    /// The record is not valid JSON: interior corruption.
    Malformed,
    /// The record is valid JSON the dialect could not interpret.
    Unparsable,
}

/// One complete record, with the reference that cites it.
#[derive(Clone, Copy, Debug)]
pub struct RawRecord<'a> {
    /// Source ID, decoded offset and length.
    pub evidence: &'a EvidenceRef,
    /// The record's bytes, without its line terminator.
    pub bytes: &'a [u8],
    /// The record's 1-based line in the decoded stream, counting every line before it:
    /// blank, oversized, malformed and skipped ones included.
    pub line: u64,
}

/// The files of one logical source: a plain file, compressed files, or several while a
/// compression or resume is in flight.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LogicalSource {
    /// The `.jsonl` file, when it exists.
    pub plain: Option<PathBuf>,
    /// The `.jsonl.zst` file, when it exists.
    pub zstd: Option<PathBuf>,
    /// The `.jsonl.gz` file, when it exists.
    pub gzip: Option<PathBuf>,
}

impl LogicalSource {
    /// A source stored as one file.
    #[cfg(test)]
    pub(crate) fn single(path: PathBuf, representation: Representation) -> Self {
        let mut files = Self::default();
        files.insert(path, representation);
        files
    }

    /// Records `path` as this source's file in `representation`.
    pub fn insert(&mut self, path: PathBuf, representation: Representation) {
        let slot = match representation {
            Representation::Plain => &mut self.plain,
            Representation::Zstd => &mut self.zstd,
            Representation::Gzip => &mut self.gzip,
        };
        *slot = Some(path);
    }

    /// Every file, in [`Representation`] preference order.
    pub fn files(&self) -> impl Iterator<Item = (&Path, Representation)> {
        [
            (&self.plain, Representation::Plain),
            (&self.zstd, Representation::Zstd),
            (&self.gzip, Representation::Gzip),
        ]
        .into_iter()
        .filter_map(|(path, representation)| Some((path.as_deref()?, representation)))
    }

    /// The representation to read: the plain file hides its compressed twins, as Codex's
    /// own discovery does, because a resume decompresses before appending; zstd, Codex's
    /// own compressed form, hides gzip.
    pub fn primary(&self) -> Option<(&Path, Representation)> {
        self.files().next()
    }

    /// The other files, when several exist.
    #[cfg(test)]
    pub(crate) fn twins(&self) -> impl Iterator<Item = (&Path, Representation)> {
        self.files().skip(1)
    }
}

/// What a source is, apart from its bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpec<'a> {
    /// The source environment registry token, such as `local`.
    pub environment: &'a str,
    /// The dialect registry token, such as `codex-rollout`.
    pub dialect: &'a str,
    /// The locator: root-relative with `/` separators and no compression suffix, or a
    /// dialect-declared stable locator.
    pub locator: &'a str,
    /// Whether `locator` is the dialect's stable locator rather than a root-relative path.
    pub stable_locator: bool,
}

impl SourceSpec<'_> {
    fn key_spec(&self) -> KeySpec {
        if self.stable_locator { SOURCE_STABLE_LOCATOR } else { SOURCE_ROOT_RELATIVE }
    }
}

/// Why a source could not be read at all.
///
/// Everything a scan can read partially is reported in its [`ManifestEntry`] instead.
#[derive(Debug, thiserror::Error)]
pub enum SourceReadError {
    /// The visitor refused further input; no partial snapshot is returned.
    #[error("source visitor stopped the scan")]
    VisitorStopped,
    /// Memory admission stopped the invocation while the scan waited for the large-record
    /// permit or was refused its charge; no partial snapshot is returned.
    #[error("memory admission stopped the scan")]
    Stopped,
    /// The logical source names no file.
    #[error("logical source has no plain or compressed file")]
    NoFile,
    /// The path could not be opened, after any retries.
    #[error("cannot open {path}: {source}")]
    Open {
        /// The path.
        path: PathBuf,
        /// The I/O error.
        #[source]
        source: io::Error,
    },
    /// The source key could not be built, such as a locator that is not a registry token.
    #[error("cannot build the source key for {locator}: {source}")]
    Key {
        /// The locator.
        locator: String,
        /// Why the key was rejected.
        #[source]
        source: KeyScopeError,
    },
    /// The source ID could not be derived.
    #[error(transparent)]
    Identity(#[from] IdentityError),
}

/// Reads one logical source under `admission`, streaming its complete records to `visit`.
///
/// The scan and its first-record checks run on the calling thread, which `admission`'s
/// worker slot pays for; a long line or a wide zstd frame takes the large-record permit.
pub fn read_source<F>(
    spec: &SourceSpec<'_>,
    files: &LogicalSource,
    options: &ReadOptions,
    admission: &MemoryAdmission,
    visit: F,
) -> Result<ManifestEntry, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    read_source_with_hooks(spec, files, options, admission, visit, &mut NoHooks)
}

/// Points where a test can change the filesystem mid-scan; a deterministic replacement,
/// truncation or brief absence cannot be produced any other way.
pub(crate) trait ScanHooks {
    /// Before each re-check of an absent path.
    fn before_retry(&mut self, _path: &Path, _attempt: u32) {}
    /// After the file is open and its snapshot extent is fixed.
    fn after_open(&mut self, _path: &Path) {}
    /// After the last record is read, before the file is re-checked.
    fn after_scan(&mut self, _path: &Path) {}
}

pub(crate) struct NoHooks;

impl ScanHooks for NoHooks {}

pub(crate) fn read_source_with_hooks<F>(
    spec: &SourceSpec<'_>,
    files: &LogicalSource,
    options: &ReadOptions,
    admission: &MemoryAdmission,
    mut visit: F,
    hooks: &mut dyn ScanHooks,
) -> Result<ManifestEntry, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    let Some((primary, primary_representation)) = files.primary() else {
        return Err(SourceReadError::NoFile);
    };
    let mut changes = Vec::new();
    let Some((path, representation, opened)) =
        open_source(files, (primary, primary_representation), options, hooks, &mut changes)?
    else {
        return Ok(vanished_entry(spec, primary, primary_representation));
    };
    let reading = Reading { spec, options, admission };
    let mut read = reading.scan_file(path, representation, opened, &mut visit, hooks)?;
    let mut skipped = None;
    if read.scan.fingerprint.is_none() {
        if let Some(mut substitute) =
            reading.read_twin_with_record(files, &read.file.path, &mut visit, hooks)?
        {
            changes.retain(|change| {
                !matches!(change, SourceChange::ReadFromOtherRepresentation { .. })
            });
            changes.push(SourceChange::ReadFromOtherRepresentation {
                primary: primary.to_owned(),
                representation: substitute.representation,
            });
            // A file that ended before its first record is unfinished, but one whose data
            // is damaged or oversized lost what it held, whatever its twin holds.
            let mut failures = std::mem::take(&mut read.scan.failures);
            failures.retain(|failure| {
                !matches!(failure, CoverageFailure::IncompleteCompressedFrame { .. })
            });
            failures.append(&mut substitute.scan.failures);
            substitute.scan.failures = failures;
            skipped = Some(read.file.path);
            read = substitute;
        }
    }
    let Scanned { file, representation, opened, snapshot_len, modified, captured_at, mut scan } =
        read;

    let twins = reading.twin_identities(
        files,
        &file.path,
        skipped.as_deref(),
        scan.fingerprint,
        &mut scan.failures,
    );
    reading.detect_changes(
        &file.path,
        representation,
        &opened,
        Snapshot { file: &file, len: snapshot_len, modified, fingerprint: scan.fingerprint },
        hooks,
        &mut changes,
    )?;

    let complete_through = scan.complete_through();
    Ok(ManifestEntry {
        source: scan.source,
        environment: spec.environment.to_owned(),
        dialect: spec.dialect.to_owned(),
        locator: spec.locator.to_owned(),
        file,
        representation,
        twins,
        file_len: snapshot_len,
        modified,
        fingerprint: scan.fingerprint,
        cutoff: Cutoff { complete_through, pending_tail: scan.pending, captured_at },
        counters: scan.counters,
        first_malformed: scan.first_malformed,
        failures: scan.failures,
        changes,
    })
}

/// One file's scan, before its twins and changes are checked.
struct Scanned {
    file: FileIdentity,
    representation: Representation,
    /// The open file, kept so the checks after the scan can examine what was read even
    /// when its path is gone.
    opened: File,
    snapshot_len: u64,
    modified: Option<SystemTime>,
    captured_at: SystemTime,
    scan: Scan,
}

/// What one source's scan reads with: the source, the limits and the admission that pays
/// for the calling thread's buffers.
#[derive(Clone, Copy)]
struct Reading<'r> {
    spec: &'r SourceSpec<'r>,
    options: &'r ReadOptions,
    admission: &'r MemoryAdmission,
}

/// What the scan saw of the file it read, to compare with the file after the scan.
#[derive(Clone, Copy)]
struct Snapshot<'s> {
    file: &'s FileIdentity,
    len: u64,
    modified: Option<SystemTime>,
    fingerprint: Option<Fingerprint>,
}

impl Reading<'_> {
    /// Streams the complete records of one open file within its extent at open.
    fn scan_file<F>(
        self,
        path: PathBuf,
        representation: Representation,
        opened: File,
        visit: &mut F,
        hooks: &mut dyn ScanHooks,
    ) -> Result<Scanned, SourceReadError>
    where
        F: FnMut(&RawRecord<'_>) -> RecordDisposition,
    {
        let captured_at = SystemTime::now();
        // The open file's own metadata fixes the extent, so a replacement of the path after
        // this point cannot change what this snapshot covers.
        let metadata = opened
            .metadata()
            .map_err(|source| SourceReadError::Open { path: path.clone(), source })?;
        hooks.after_open(&path);

        let snapshot_len = metadata.len();
        let file = FileIdentity { device: device_of(&metadata), inode: inode_of(&metadata), path };
        let modified = metadata.modified().ok();

        let mut scan = Scan {
            counters: RecordCounters::default(),
            failures: Vec::new(),
            first_malformed: None,
            fingerprint: None,
            source: None,
            offset: 0,
            lines: 0,
            pending: None,
        };
        let mut reader = reader_for(&opened, snapshot_len, representation, self.admission);
        scan.run(&mut reader, self, visit, representation)?;
        drop(reader);
        hooks.after_scan(&file.path);
        Ok(Scanned { file, representation, opened, snapshot_len, modified, captured_at, scan })
    }

    /// Reads, in place of a file that held no complete record, the first other discovered
    /// file of the source that holds one; `None` when none does.
    ///
    /// The file read yields no record that file lacks, as while a decompressor or a Codex
    /// resume has created the plain file and not yet written its first line. Without this,
    /// every such twin would go unread. Nothing was visited yet, since a record is visited
    /// only after the first complete one fixes the source ID.
    fn read_twin_with_record<F>(
        self,
        files: &LogicalSource,
        read: &Path,
        visit: &mut F,
        hooks: &mut dyn ScanHooks,
    ) -> Result<Option<Scanned>, SourceReadError>
    where
        F: FnMut(&RawRecord<'_>) -> RecordDisposition,
    {
        for (twin, representation) in files.files().filter(|(path, _)| *path != read) {
            let first = self.first_record_fingerprint(twin, representation);
            if !matches!(first, FirstRecord::Record(_)) {
                continue;
            }
            let Some(opened) = open_once(twin)? else { continue };
            return self.scan_file(twin.to_owned(), representation, opened, visit, hooks).map(Some);
        }
        Ok(None)
    }
}

/// Opens the file to read: the primary with retries, then, when it is gone, its
/// [`fallbacks`] in order, recording which one was read instead. `None` when no file of
/// the source can be opened.
fn open_source(
    files: &LogicalSource,
    primary: (&Path, Representation),
    options: &ReadOptions,
    hooks: &mut dyn ScanHooks,
    changes: &mut Vec<SourceChange>,
) -> Result<Option<(PathBuf, Representation, File)>, SourceReadError> {
    let (primary, primary_representation) = primary;
    if let Some(file) = open_with_retry(primary, options, hooks, changes)? {
        return Ok(Some((primary.to_owned(), primary_representation, file)));
    }
    for fallback in fallbacks(files, primary) {
        if let Some(file) = fallback.open()? {
            changes.push(SourceChange::ReadFromOtherRepresentation {
                primary: primary.to_owned(),
                representation: fallback.representation,
            });
            return Ok(Some((fallback.path, fallback.representation, file)));
        }
    }
    Ok(None)
}

/// A file the reader may open for a source.
struct Candidate {
    path: PathBuf,
    representation: Representation,
    /// Whether discovery recorded, and so vetted, this file.
    discovered: bool,
}

impl Candidate {
    /// Opens the file; `None` when it is absent or, for a file discovery never saw, not a
    /// regular file.
    fn open(&self) -> Result<Option<File>, SourceReadError> {
        if self.discovered { open_once(&self.path) } else { open_regular_file(&self.path) }
    }
}

/// The files to try, in preference order, when a source's primary is gone: each other
/// representation's discovered file or, when discovery found none, the primary's sibling
/// in that representation, such as the `.jsonl.gz` that `gzip` wrote after discovery.
fn fallbacks(files: &LogicalSource, primary: &Path) -> Vec<Candidate> {
    Representation::ALL
        .into_iter()
        .filter_map(|representation| {
            let discovered = files.files().find(|(_, held)| *held == representation);
            let (path, discovered) = match discovered {
                Some((path, _)) => (path.to_owned(), true),
                None => (sibling(primary, representation)?, false),
            };
            (path != primary).then_some(Candidate { path, representation, discovered })
        })
        .collect()
}

/// Decodes the files of a source in reading order until `look` accepts one, for a look
/// at its first records outside a snapshot: the primary, then the files [`read_source`]
/// falls back to, opened under the same rules. A file that is absent, cannot be opened or
/// decoded, or that `look` rejects, as when it has no complete first record, is skipped,
/// so one damaged file never stops the look. `None` when no file is accepted.
///
/// `look` reads lines by the scan's rule ([`PeekLines`]), under `admission` as a scan
/// does: the calling thread's slot pays for the decoder and a line up to 4 MiB, and a
/// longer line or a wider zstd frame takes the large-record permit.
pub fn peek<T>(
    files: &LogicalSource,
    admission: &MemoryAdmission,
    mut look: impl FnMut(&mut PeekLines<'_>) -> Option<T>,
) -> Option<T> {
    let (primary, representation) = files.primary()?;
    let first = Candidate { path: primary.to_owned(), representation, discovered: true };
    std::iter::once(first).chain(fallbacks(files, primary)).find_map(|candidate| {
        let file = candidate.open().ok()??;
        let reader = decode(file, candidate.representation, admission).ok()?;
        look(&mut PeekLines { reader: reader.take(u64::MAX), buffer: LineBuffer::new(admission) })
    })
}

/// The decoded lines of one file, for a [`peek`] at its first records: each read follows
/// the scan's line rule and buffer, within a window of decoded bytes the look may set.
pub struct PeekLines<'a> {
    reader: io::Take<Box<dyn BufRead + 'a>>,
    buffer: LineBuffer<'a>,
}

/// One line a [`PeekLines`] read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeekLine<'b> {
    /// The stream or the window ended exactly at a line boundary.
    End,
    /// A newline-terminated line, without its terminator.
    Complete(&'b [u8]),
    /// A newline-terminated line longer than the read's limit, whose bytes were skipped.
    Oversized,
    /// Bytes after the last terminator, up to the end of the stream or the window: the
    /// bytes, or `None` when they passed the read's limit and were skipped.
    Pending(Option<&'b [u8]>),
}

impl PeekLines<'_> {
    /// Lets the following reads consume at most `bytes` more decoded bytes.
    pub fn set_window(&mut self, bytes: u64) {
        self.reader.set_limit(bytes);
    }

    /// The decoded bytes the window still allows.
    pub fn window(&self) -> u64 {
        self.reader.limit()
    }

    /// Reads the next line by the scan's rule, buffering at most `limit` bytes of it.
    ///
    /// # Errors
    ///
    /// The stream's read or decoding error, or one that says admission stopped the
    /// invocation while the read waited for the large-record permit.
    pub fn next_line(&mut self, limit: usize) -> io::Result<PeekLine<'_>> {
        self.buffer.reset();
        Ok(match read_line(&mut self.reader, &mut self.buffer, limit)? {
            Line::Eof => PeekLine::End,
            Line::Complete { .. } => PeekLine::Complete(self.buffer.bytes()),
            Line::Oversized { .. } => PeekLine::Oversized,
            Line::Pending { oversized: true, .. } => PeekLine::Pending(None),
            Line::Pending { oversized: false, .. } => PeekLine::Pending(Some(self.buffer.bytes())),
        })
    }
}

/// Opens a file discovery never saw, only when it is a regular file, as discovery requires.
/// A symbolic link could leave the declared roots or reach a source already read under
/// another name, and opening a FIFO would block the worker. `None` when the path is absent,
/// is not a regular file, or names a different file once open.
///
/// A regular file replaced by a FIFO between the check and the open would still block;
/// that needs a writer to the root acting in that window.
fn open_regular_file(path: &Path) -> Result<Option<File>, SourceReadError> {
    let checked = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(SourceReadError::Open { path: path.to_owned(), source }),
    };
    let Some(file) = open_once(path)? else { return Ok(None) };
    let opened = file
        .metadata()
        .map_err(|source| SourceReadError::Open { path: path.to_owned(), source })?;
    let same = opened.is_file()
        && device_of(&opened) == device_of(&checked)
        && inode_of(&opened) == inode_of(&checked);
    Ok(same.then_some(file))
}

/// The file of the same source in `representation`, such as `a/b.jsonl.zst` for
/// `a/b.jsonl`; `None` when the name is not UTF-8 or names no representation.
fn sibling(path: &Path, representation: Representation) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    let base = name.strip_suffix(Representation::of_path(path)?.suffix())?;
    Some(path.with_file_name(format!("{base}{}", representation.suffix())))
}

/// The snapshot of a source none of whose files exists any more.
fn vanished_entry(
    spec: &SourceSpec<'_>,
    path: &Path,
    representation: Representation,
) -> ManifestEntry {
    ManifestEntry {
        source: None,
        environment: spec.environment.to_owned(),
        dialect: spec.dialect.to_owned(),
        locator: spec.locator.to_owned(),
        file: FileIdentity { path: path.to_owned(), device: None, inode: None },
        representation,
        twins: Vec::new(),
        file_len: 0,
        modified: None,
        fingerprint: None,
        cutoff: Cutoff { complete_through: 0, pending_tail: None, captured_at: SystemTime::now() },
        counters: RecordCounters::default(),
        first_malformed: None,
        failures: Vec::new(),
        changes: vec![SourceChange::Vanished],
    }
}

/// The running state of one scan.
struct Scan {
    counters: RecordCounters,
    failures: Vec<CoverageFailure>,
    first_malformed: Option<EvidenceRef>,
    fingerprint: Option<Fingerprint>,
    source: Option<StoredIdentity>,
    offset: u64,
    /// Lines read so far, whatever their disposition.
    lines: u64,
    pending: Option<PendingTail>,
}

impl Scan {
    fn complete_through(&self) -> u64 {
        match self.pending {
            Some(tail) => tail.offset,
            None => self.offset,
        }
    }

    fn run<F>(
        &mut self,
        reader: &mut dyn BufRead,
        reading: Reading<'_>,
        visit: &mut F,
        representation: Representation,
    ) -> Result<(), SourceReadError>
    where
        F: FnMut(&RawRecord<'_>) -> RecordDisposition,
    {
        let Reading { spec, options, admission } = reading;
        let mut buffer = LineBuffer::new(admission);
        loop {
            // Once the visitor has returned, a grown buffer shrinks back and releases the
            // large-record permit.
            buffer.reset();
            let line = match read_line(reader, &mut buffer, options.max_record_bytes) {
                Ok(line) => line,
                Err(error) if is_stopped(&error) => return Err(SourceReadError::Stopped),
                Err(error) => {
                    self.failures.push(read_failure(&error, self.offset, representation));
                    return Ok(());
                }
            };
            let bytes = buffer.bytes();
            match line {
                Line::Eof => return Ok(()),
                Line::Pending { length, oversized } => {
                    if oversized {
                        self.counters.oversized = self.counters.oversized.saturating_add(1);
                        self.failures
                            .push(CoverageFailure::Oversized { offset: self.offset, length });
                    }
                    self.pending = Some(PendingTail { offset: self.offset, length });
                    self.offset = self.offset.saturating_add(length);
                    return Ok(());
                }
                Line::Oversized { length } => {
                    self.lines = self.lines.saturating_add(1);
                    self.counters.oversized = self.counters.oversized.saturating_add(1);
                    self.failures.push(CoverageFailure::Oversized { offset: self.offset, length });
                    self.advance(length);
                }
                Line::Complete { length } => {
                    self.lines = self.lines.saturating_add(1);
                    if bytes.iter().all(u8::is_ascii_whitespace) {
                        self.counters.blank_lines = self.counters.blank_lines.saturating_add(1);
                        self.advance(length);
                        continue;
                    }
                    if self.fingerprint.is_none() {
                        let fingerprint = Fingerprint::of(bytes);
                        self.fingerprint = Some(fingerprint);
                        self.source = Some(source_identity(spec, fingerprint)?);
                    }
                    if self.source.is_none() {
                        return Ok(());
                    }
                    let evidence = EvidenceRef::new(0, self.offset, length);
                    let disposition =
                        visit(&RawRecord { evidence: &evidence, bytes, line: self.lines });
                    self.counters.records = self.counters.records.saturating_add(1);
                    match disposition {
                        RecordDisposition::Stop => return Err(SourceReadError::VisitorStopped),
                        RecordDisposition::Decoded => {
                            self.counters.decoded = self.counters.decoded.saturating_add(1);
                        }
                        RecordDisposition::Skipped => {
                            self.counters.skipped = self.counters.skipped.saturating_add(1);
                        }
                        RecordDisposition::Malformed => {
                            self.counters.malformed = self.counters.malformed.saturating_add(1);
                            if self.first_malformed.is_none() {
                                self.first_malformed = Some(evidence);
                            }
                        }
                        RecordDisposition::Unparsable => {
                            self.counters.unparsable = self.counters.unparsable.saturating_add(1);
                        }
                    }
                    self.advance(length);
                }
            }
        }
    }

    /// Steps past a complete line and its terminator.
    fn advance(&mut self, length: u64) {
        self.offset = self.offset.saturating_add(length).saturating_add(1);
    }
}

fn source_identity(
    spec: &SourceSpec<'_>,
    fingerprint: Fingerprint,
) -> Result<StoredIdentity, SourceReadError> {
    let key = spec
        .key_spec()
        .key(vec![
            KeyComponent::text(spec.environment),
            KeyComponent::text(spec.dialect),
            KeyComponent::text(spec.locator),
            KeyComponent::text(fingerprint.to_base32()),
        ])
        .map_err(|source| SourceReadError::Key { locator: spec.locator.to_owned(), source })?;
    Ok(StoredIdentity::derive(key.key)?)
}

fn reader_for<'a>(
    file: &'a File,
    extent: u64,
    representation: Representation,
    admission: &'a MemoryAdmission,
) -> Box<dyn BufRead + 'a> {
    // A decoder is only built here; a failure is a corrupt or empty stream, which the scan
    // reports as a read failure at offset 0.
    decode(file.take(extent), representation, admission)
        .unwrap_or_else(|error| Box::new(FailingReader(Some(error))))
}

/// The read buffer [`decode`] wraps every decoder in.
pub const READ_BUFFER_BYTES: usize = 128 * 1024;

/// Decodes `stored`, a source's bytes in `representation`, into a buffered reader of its
/// records under `admission`: the one mapping from a representation to its decoder, and
/// the only way the reader opens one.
///
/// zstd and gzip decoding read every frame or member, so files concatenated by `cat` or
/// appended to by a streaming compressor decode whole. zstd decodes through the reader's
/// own frame loop, which gives a frame whose window passes the worker slot's 8 MiB a
/// second decoder under `admission`'s large-record permit (`frames`).
///
/// # Errors
///
/// A zstd decoder that cannot be created. Damaged data surfaces from the reader instead.
pub fn decode<'a, R: Read + 'a>(
    stored: R,
    representation: Representation,
    admission: &'a MemoryAdmission,
) -> io::Result<Box<dyn BufRead + 'a>> {
    Ok(match representation {
        Representation::Plain => Box::new(BufReader::with_capacity(READ_BUFFER_BYTES, stored)),
        Representation::Zstd => Box::new(BufReader::with_capacity(
            READ_BUFFER_BYTES,
            frames::ZstdFrames::new(stored, admission)?,
        )),
        Representation::Gzip => Box::new(BufReader::with_capacity(
            READ_BUFFER_BYTES,
            flate2::read::MultiGzDecoder::new(stored),
        )),
    })
}

/// The error a read returns once admission has stopped the invocation, while it waited
/// for the large-record permit or when the permit's charge was refused.
#[derive(Debug, thiserror::Error)]
#[error("memory admission stopped the read")]
struct AdmissionStopped;

fn stopped() -> io::Error {
    io::Error::other(AdmissionStopped)
}

/// Whether `error` is [`stopped`]'s, which ends a read without being a source failure.
fn is_stopped(error: &io::Error) -> bool {
    error.get_ref().is_some_and(<dyn std::error::Error + Send + Sync>::is::<AdmissionStopped>)
}

/// A reader that yields one error, so a decoder that cannot start is reported like any
/// other read failure rather than by a separate path.
struct FailingReader(Option<io::Error>);

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        match self.0.take() {
            Some(error) => Err(error),
            None => Ok(0),
        }
    }
}

impl BufRead for FailingReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self.0.take() {
            Some(error) => Err(error),
            None => Ok(&[]),
        }
    }

    fn consume(&mut self, _amount: usize) {}
}

/// Classifies an error that ended a scan. zstd reports damaged data as
/// [`io::ErrorKind::Other`] and flate2 as [`io::ErrorKind::InvalidInput`] or
/// [`io::ErrorKind::InvalidData`]; both end a truncated stream with
/// [`io::ErrorKind::UnexpectedEof`].
fn read_failure(error: &io::Error, offset: u64, representation: Representation) -> CoverageFailure {
    let kind = error.kind();
    let compressed = representation != Representation::Plain;
    let damaged = [io::ErrorKind::Other, io::ErrorKind::InvalidInput, io::ErrorKind::InvalidData];
    if compressed && kind == io::ErrorKind::UnexpectedEof {
        CoverageFailure::IncompleteCompressedFrame { decoded_offset: offset }
    } else if compressed && damaged.contains(&kind) {
        CoverageFailure::CorruptCompressedData {
            decoded_offset: offset,
            message: error.to_string(),
        }
    } else {
        CoverageFailure::ReadError { decoded_offset: offset, kind }
    }
}

/// What one read of a line produced.
enum Line {
    /// The stream ended exactly at a record boundary.
    Eof,
    /// A newline-terminated line of `length` bytes, in the caller's buffer.
    Complete { length: u64 },
    /// A newline-terminated line too long to buffer; its bytes were skipped.
    Oversized { length: u64 },
    /// Bytes after the last terminator: an unfinished tail.
    Pending { length: u64, oversized: bool },
}

/// One reader's line buffer under memory admission.
///
/// It starts at [`ReadOptions::RETAINED_LINE_CAPACITY`] and grows only by doubling, from
/// capacity `C` to `C′ = min(2 × C, limit)` with `reserve_exact`, so its capacity never
/// passes the read's limit and its need is the model's `line_need(C′)`. A capacity past the
/// worker slot's 4 MiB takes the large-record permit for that need before the buffer
/// grows, and [`LineBuffer::reset`] shrinks the buffer and releases the permit once the
/// line is no longer needed.
struct LineBuffer<'a> {
    bytes: Vec<u8>,
    admission: &'a MemoryAdmission,
    permit: Option<LargeRecord<'a>>,
}

impl<'a> LineBuffer<'a> {
    /// An empty buffer at the retained capacity.
    fn new(admission: &'a MemoryAdmission) -> Self {
        Self {
            bytes: Vec::with_capacity(ReadOptions::RETAINED_LINE_CAPACITY),
            admission,
            permit: None,
        }
    }

    /// The line read so far.
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The buffer's capacity.
    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// Whether this buffer holds a share of the large-record permit.
    #[cfg(test)]
    const fn holds_permit(&self) -> bool {
        self.permit.is_some()
    }

    /// Drops the line read so far, keeping the capacity.
    fn clear(&mut self) {
        self.bytes.clear();
    }

    /// Appends `chunk`, doubling the capacity until it fits within `limit`. The caller
    /// has checked that the line stays within `limit`.
    fn extend(&mut self, chunk: &[u8], limit: usize) -> io::Result<()> {
        let needed = self.bytes.len().saturating_add(chunk.len());
        while self.bytes.capacity() < needed {
            let capacity = self.bytes.capacity();
            let next = usize::try_from(next_line_capacity(as_bytes(capacity), as_bytes(limit)))
                .unwrap_or(usize::MAX);
            if next <= capacity {
                // Only a caller that passed its own limit gets here; grow to fit.
                self.bytes.reserve_exact(needed - self.bytes.len());
                break;
            }
            if as_bytes(next) > SLOT_LINE_CAPACITY {
                let need = line_need(as_bytes(next));
                match &mut self.permit {
                    Some(permit) => {
                        if !permit.raise(need) {
                            return Err(stopped());
                        }
                    }
                    None => {
                        self.permit = Some(self.admission.large_record(need).ok_or_else(stopped)?);
                    }
                }
            }
            self.bytes.reserve_exact(next - self.bytes.len());
        }
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }

    /// Drops the line, shrinks a grown buffer back to the retained capacity so one huge
    /// record does not pin the worker, and releases the large-record permit.
    fn reset(&mut self) {
        self.bytes.clear();
        if self.bytes.capacity() > ReadOptions::RETAINED_LINE_CAPACITY {
            self.bytes.shrink_to(ReadOptions::RETAINED_LINE_CAPACITY);
        }
        self.permit = None;
    }
}

/// A byte count as the cost model's `u64`.
fn as_bytes(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// Reads one line into `buffer` without its terminator, buffering at most `limit` bytes.
fn read_line(
    reader: &mut dyn BufRead,
    buffer: &mut LineBuffer<'_>,
    limit: usize,
) -> io::Result<Line> {
    let mut length: u64 = 0;
    let mut oversized = false;
    loop {
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if available.is_empty() {
            return Ok(if length == 0 { Line::Eof } else { Line::Pending { length, oversized } });
        }
        if let Some(newline) = memchr::memchr(b'\n', available) {
            if !oversized {
                if buffer.bytes.len().saturating_add(newline) > limit {
                    oversized = true;
                    buffer.clear();
                } else if let Some(line) = available.get(..newline) {
                    buffer.extend(line, limit)?;
                }
            }
            length = length.saturating_add(newline as u64);
            reader.consume(newline.saturating_add(1));
            return Ok(if oversized {
                Line::Oversized { length }
            } else {
                Line::Complete { length }
            });
        }
        let taken = available.len();
        if !oversized {
            if buffer.bytes.len().saturating_add(taken) > limit {
                oversized = true;
                buffer.clear();
            } else {
                buffer.extend(available, limit)?;
            }
        }
        length = length.saturating_add(taken as u64);
        reader.consume(taken);
    }
}

/// Opens `path`, re-checking an absent path; `None` when it stays absent.
fn open_with_retry(
    path: &Path,
    options: &ReadOptions,
    hooks: &mut dyn ScanHooks,
    changes: &mut Vec<SourceChange>,
) -> Result<Option<File>, SourceReadError> {
    let mut attempts = 0;
    loop {
        match File::open(path) {
            Ok(file) => {
                if attempts > 0 {
                    changes.push(SourceChange::BrieflyAbsent { attempts });
                }
                return Ok(Some(file));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if attempts >= options.absent_retries {
                    return Ok(None);
                }
                attempts = attempts.saturating_add(1);
                hooks.before_retry(path, attempts);
                if !options.absent_retry_delay.is_zero() {
                    std::thread::sleep(options.absent_retry_delay);
                }
            }
            Err(source) => return Err(SourceReadError::Open { path: path.to_owned(), source }),
        }
    }
}

/// Opens `path` once; `None` when it does not exist.
fn open_once(path: &Path) -> Result<Option<File>, SourceReadError> {
    match File::open(path) {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(SourceReadError::Open { path: path.to_owned(), source }),
    }
}

impl Reading<'_> {
    /// Compares the source after its scan with its snapshot: the path's identity, length
    /// and modification time, and its first record. When the path is gone after its
    /// retries, the open file is examined instead, since it is what the snapshot read.
    fn detect_changes(
        self,
        path: &Path,
        representation: Representation,
        opened: &File,
        snapshot: Snapshot<'_>,
        hooks: &mut dyn ScanHooks,
        changes: &mut Vec<SourceChange>,
    ) -> Result<(), SourceReadError> {
        let options = self.options;
        let mut attempts = 0;
        let metadata = loop {
            match std::fs::metadata(path) {
                Ok(metadata) => break Some(metadata),
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound
                        && attempts < options.absent_retries =>
                {
                    attempts = attempts.saturating_add(1);
                    hooks.before_retry(path, attempts);
                    if !options.absent_retry_delay.is_zero() {
                        std::thread::sleep(options.absent_retry_delay);
                    }
                }
                Err(_) => break None,
            }
        };
        let fingerprint = snapshot.fingerprint;
        let Some(metadata) = metadata else {
            // A compressor that finished or an agent that expired the file removed the
            // path, but the open file supplied the snapshot, so only a change to it loses
            // records.
            let held = opened
                .metadata()
                .map_err(|source| SourceReadError::Open { path: path.to_owned(), source })?;
            compare_extent(&held, snapshot.len, snapshot.modified, changes);
            let first = self.first_record_of(opened, representation);
            if fingerprint.is_some_and(|ours| first != FirstRecord::Record(ours)) {
                changes.push(SourceChange::FirstRecordChanged);
            }
            changes.push(SourceChange::RemovedAfterScan);
            return Ok(());
        };
        if attempts > 0
            && !changes.iter().any(|change| matches!(change, SourceChange::BrieflyAbsent { .. }))
        {
            changes.push(SourceChange::BrieflyAbsent { attempts });
        }
        if device_of(&metadata) != snapshot.file.device
            || inode_of(&metadata) != snapshot.file.inode
        {
            changes.push(SourceChange::Replaced);
            return Ok(());
        }
        compare_extent(&metadata, snapshot.len, snapshot.modified, changes);
        let first = self.first_record_fingerprint(path, representation);
        if fingerprint.is_some_and(|ours| first != FirstRecord::Record(ours)) {
            changes.push(SourceChange::FirstRecordChanged);
        }
        Ok(())
    }

    /// Re-reads a file's first record by path, to check that the bytes the source ID
    /// covers did not change or to compare a twin with the file read.
    fn first_record_fingerprint(self, path: &Path, representation: Representation) -> FirstRecord {
        match File::open(path) {
            Ok(file) => self.first_record_of(&file, representation),
            Err(error) if error.kind() == io::ErrorKind::NotFound => FirstRecord::NotYet,
            Err(_) => FirstRecord::Unreadable,
        }
    }

    /// The first record of an open file, read from its start through [`decode`] and the
    /// scan's line buffer, on the calling worker's slot. A read that admission stops is
    /// unreadable; the invocation reports the refusal instead.
    fn first_record_of(self, mut file: &File, representation: Representation) -> FirstRecord {
        if file.seek(SeekFrom::Start(0)).is_err() {
            return FirstRecord::Unreadable;
        }
        let Ok(mut reader) = decode(file, representation, self.admission) else {
            return FirstRecord::Unreadable;
        };
        let mut buffer = LineBuffer::new(self.admission);
        let mut passed_oversized = false;
        // An oversized record is a record that cannot be compared, unlike an early end.
        let ended = |passed_oversized: bool| {
            if passed_oversized { FirstRecord::Unreadable } else { FirstRecord::NotYet }
        };
        loop {
            buffer.reset();
            match read_line(&mut reader, &mut buffer, self.options.max_record_bytes) {
                Ok(Line::Complete { .. }) => {
                    if !buffer.bytes().iter().all(u8::is_ascii_whitespace) {
                        return FirstRecord::Record(Fingerprint::of(buffer.bytes()));
                    }
                }
                Ok(Line::Oversized { .. }) => passed_oversized = true,
                Ok(Line::Pending { oversized, .. }) => return ended(passed_oversized || oversized),
                Ok(Line::Eof) => return ended(passed_oversized),
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    return ended(passed_oversized);
                }
                Err(_) => return FirstRecord::Unreadable,
            }
        }
    }

    /// Checks each other discovered file of the source against the file read, except a
    /// primary read in place of which `skipped` names, whose own failures were kept.
    ///
    /// A twin whose first record matches is recorded as verified, one whose first record
    /// differs is a different logical source this scan did not read, and one whose first
    /// record cannot be read is unreadable; both are coverage failures. A twin with no
    /// complete record yet is neither: it is gone with the primary, empty, or a
    /// compressor's unfinished output, which ends early rather than failing to decode.
    fn twin_identities(
        self,
        files: &LogicalSource,
        read: &Path,
        skipped: Option<&Path>,
        fingerprint: Option<Fingerprint>,
        failures: &mut Vec<CoverageFailure>,
    ) -> Vec<FileIdentity> {
        let mut identities = Vec::new();
        for (twin, representation) in
            files.files().filter(|(path, _)| *path != read && Some(*path) != skipped)
        {
            let theirs = self.first_record_fingerprint(twin, representation);
            let path = twin.to_owned();
            let locator = self.spec.locator.to_owned();
            match theirs {
                FirstRecord::NotYet => {}
                FirstRecord::Unreadable => {
                    failures.push(CoverageFailure::UnreadableTwin { path, locator });
                }
                FirstRecord::Record(theirs) if fingerprint != Some(theirs) => {
                    failures.push(CoverageFailure::TwinFingerprintMismatch { path, locator });
                }
                FirstRecord::Record(_) => {
                    let metadata = std::fs::metadata(twin).ok();
                    identities.push(FileIdentity {
                        path,
                        device: metadata.as_ref().and_then(device_of_opt),
                        inode: metadata.as_ref().and_then(inode_of_opt),
                    });
                }
            }
        }
        identities
    }
}

/// Records a change in a file's length or, at the same length, its modification time.
fn compare_extent(
    metadata: &Metadata,
    snapshot_len: u64,
    snapshot_modified: Option<SystemTime>,
    changes: &mut Vec<SourceChange>,
) {
    let observed_len = metadata.len();
    if observed_len < snapshot_len {
        changes.push(SourceChange::Truncated { snapshot_len, observed_len });
    } else if observed_len > snapshot_len {
        changes.push(SourceChange::GrewBeyondCutoff { observed_len });
    } else if metadata.modified().ok() != snapshot_modified {
        changes.push(SourceChange::ModifiedInPlace);
    }
}

/// What re-reading a file's first record found.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FirstRecord {
    /// The first complete record within the size bound, found as the scan finds it:
    /// blank and oversized lines before it are passed over.
    Record(Fingerprint),
    /// No complete record yet: the file is absent, or its stream ends before any record
    /// or oversized line, cleanly, with a pending line, or inside a compressed frame or
    /// member. Every proper prefix of a valid gzip or zstd stream ends this way, as a
    /// compressor's unfinished output does.
    NotYet,
    /// The file cannot be opened, its data does not decode, or it holds an oversized record
    /// and no record within the bound, so its first record cannot be compared.
    Unreadable,
}

// The Option is the platform-independent shape: Windows reports neither value.
#[cfg(unix)]
#[expect(clippy::unnecessary_wraps, reason = "the non-unix arm returns None")]
fn device_of(metadata: &Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt as _;
    Some(metadata.dev())
}

#[cfg(unix)]
#[expect(clippy::unnecessary_wraps, reason = "the non-unix arm returns None")]
fn inode_of(metadata: &Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt as _;
    Some(metadata.ino())
}

// Windows exposes a file's volume and index only through `windows_by_handle`, which is
// unstable, so identity there rests on path, length and modification time.
#[cfg(not(unix))]
fn device_of(_metadata: &Metadata) -> Option<u64> {
    None
}

#[cfg(not(unix))]
fn inode_of(_metadata: &Metadata) -> Option<u64> {
    None
}

fn device_of_opt(metadata: &Metadata) -> Option<u64> {
    device_of(metadata)
}

fn inode_of_opt(metadata: &Metadata) -> Option<u64> {
    inode_of(metadata)
}

mod frames;

#[cfg(test)]
mod tests;
