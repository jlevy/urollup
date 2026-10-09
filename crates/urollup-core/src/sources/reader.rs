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
//!   [`SourceChange::BrieflyAbsent`] rather than counted as a lost source. An in-place
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
//! - **Compression.** `.jsonl`, `.jsonl.zst` and `.jsonl.gz` decode to the same bytes, so
//!   evidence offsets are decoded offsets and a `.jsonl` file and its compressed twins are
//!   one logical source with one `src-` ID. Each other file of the source is a verified
//!   twin when its first record matches the file read and a different, unread source when
//!   it differs; a file with no complete first record yet, such as a compressor's
//!   unfinished output, is neither. When the file read has no complete record but another
//!   file of the source has one, that file is read instead. [`decode`] is the one place a
//!   representation maps to a decoder. A compressed stream that ends inside a frame or
//!   member is [`CoverageFailure::IncompleteCompressedFrame`], distinct from
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
    pub fn single(path: PathBuf, representation: Representation) -> Self {
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
    pub fn twins(&self) -> impl Iterator<Item = (&Path, Representation)> {
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

/// Reads one logical source, streaming its complete records to `visit`.
pub fn read_source<F>(
    spec: &SourceSpec<'_>,
    files: &LogicalSource,
    options: &ReadOptions,
    visit: F,
) -> Result<ManifestEntry, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    read_source_with_hooks(spec, files, options, visit, &mut NoHooks)
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
    let mut read = scan_file(spec, path, representation, &opened, options, &mut visit, hooks)?;
    if read.scan.fingerprint.is_none() {
        if let Some(substitute) =
            read_twin_with_record(spec, files, &read.file.path, options, &mut visit, hooks)?
        {
            changes = vec![SourceChange::ReadFromOtherRepresentation {
                primary: primary.to_owned(),
                representation: substitute.representation,
            }];
            read = substitute;
        }
    }
    let Scanned { file, representation, snapshot_len, modified, captured_at, mut scan } = read;

    let twins = twin_identities(files, &file.path, scan.fingerprint, spec, &mut scan.failures);
    detect_changes(
        &file.path,
        representation,
        &file,
        snapshot_len,
        modified,
        scan.fingerprint,
        options,
        hooks,
        &mut changes,
    );

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
    snapshot_len: u64,
    modified: Option<SystemTime>,
    captured_at: SystemTime,
    scan: Scan,
}

/// Streams the complete records of one open file within its extent at open.
fn scan_file<F>(
    spec: &SourceSpec<'_>,
    path: PathBuf,
    representation: Representation,
    opened: &File,
    options: &ReadOptions,
    visit: &mut F,
    hooks: &mut dyn ScanHooks,
) -> Result<Scanned, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    let captured_at = SystemTime::now();
    // The open file's own metadata fixes the extent, so a replacement of the path after
    // this point cannot change what this snapshot covers.
    let metadata =
        opened.metadata().map_err(|source| SourceReadError::Open { path: path.clone(), source })?;
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
        pending: None,
    };
    let mut reader = reader_for(opened, snapshot_len, representation);
    scan.run(&mut reader, spec, options, visit, representation)?;
    hooks.after_scan(&file.path);
    Ok(Scanned { file, representation, snapshot_len, modified, captured_at, scan })
}

/// Reads, in place of a file that held no complete record, the first other discovered
/// file of the source that holds one; `None` when none does.
///
/// The file read holds nothing that file lacks, as while a decompressor or a Codex resume
/// has created the plain file and not yet written its first line. Without this, every
/// such twin would be accepted unread. Nothing was visited yet, since a record is visited
/// only after the first complete one fixes the source ID.
fn read_twin_with_record<F>(
    spec: &SourceSpec<'_>,
    files: &LogicalSource,
    read: &Path,
    options: &ReadOptions,
    visit: &mut F,
    hooks: &mut dyn ScanHooks,
) -> Result<Option<Scanned>, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    for (twin, representation) in files.files().filter(|(path, _)| *path != read) {
        if first_record_fingerprint(twin, representation).is_none() {
            continue;
        }
        let Some(opened) = open_once(twin)? else { continue };
        return scan_file(spec, twin.to_owned(), representation, &opened, options, visit, hooks)
            .map(Some);
    }
    Ok(None)
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

/// A file the reader may open in place of a source's primary.
struct Fallback {
    path: PathBuf,
    representation: Representation,
    /// Whether discovery recorded, and so vetted, this file.
    discovered: bool,
}

impl Fallback {
    /// Opens the file; `None` when it is absent or, for a file discovery never saw, not a
    /// regular file.
    fn open(&self) -> Result<Option<File>, SourceReadError> {
        if self.discovered { open_once(&self.path) } else { open_regular_file(&self.path) }
    }
}

/// The files to try, in preference order, when a source's primary is gone: each other
/// representation's discovered file or, when discovery found none, the primary's sibling
/// in that representation, such as the `.jsonl.gz` that `gzip` wrote after discovery.
fn fallbacks(files: &LogicalSource, primary: &Path) -> Vec<Fallback> {
    Representation::ALL
        .into_iter()
        .filter_map(|representation| {
            let discovered = files.files().find(|(_, held)| *held == representation);
            let (path, discovered) = match discovered {
                Some((path, _)) => (path.to_owned(), true),
                None => (sibling(primary, representation)?, false),
            };
            (path != primary).then_some(Fallback { path, representation, discovered })
        })
        .collect()
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
        spec: &SourceSpec<'_>,
        options: &ReadOptions,
        visit: &mut F,
        representation: Representation,
    ) -> Result<(), SourceReadError>
    where
        F: FnMut(&RawRecord<'_>) -> RecordDisposition,
    {
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            shrink_line_buffer(&mut buffer);
            let line = match read_line(reader, &mut buffer, options.max_record_bytes) {
                Ok(line) => line,
                Err(error) => {
                    self.failures.push(read_failure(&error, self.offset, representation));
                    return Ok(());
                }
            };
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
                    self.counters.oversized = self.counters.oversized.saturating_add(1);
                    self.failures.push(CoverageFailure::Oversized { offset: self.offset, length });
                    self.advance(length);
                }
                Line::Complete { length } => {
                    if buffer.iter().all(u8::is_ascii_whitespace) {
                        self.counters.blank_lines = self.counters.blank_lines.saturating_add(1);
                        self.advance(length);
                        continue;
                    }
                    if self.fingerprint.is_none() {
                        let fingerprint = Fingerprint::of(&buffer);
                        self.fingerprint = Some(fingerprint);
                        self.source = Some(source_identity(spec, fingerprint)?);
                    }
                    if self.source.is_none() {
                        return Ok(());
                    }
                    let evidence = EvidenceRef::new(0, self.offset, length);
                    let disposition = visit(&RawRecord { evidence: &evidence, bytes: &buffer });
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

fn reader_for(file: &File, extent: u64, representation: Representation) -> Box<dyn BufRead + '_> {
    // A decoder is only built here; a failure is a corrupt or empty stream, which the scan
    // reports as a read failure at offset 0.
    decode(file.take(extent), representation)
        .unwrap_or_else(|error| Box::new(FailingReader(Some(error))))
}

/// Decodes `stored`, a source's bytes in `representation`, into a buffered reader of its
/// records: the one mapping from a representation to its decoder.
///
/// zstd and gzip decoding read every frame or member, so files concatenated by `cat` or
/// appended to by a streaming compressor decode whole.
///
/// # Errors
///
/// A zstd decoder that cannot be created. Damaged data surfaces from the reader instead.
pub fn decode<'a, R: Read + 'a>(
    stored: R,
    representation: Representation,
) -> io::Result<Box<dyn BufRead + 'a>> {
    const CAPACITY: usize = 128 * 1024;
    Ok(match representation {
        Representation::Plain => Box::new(BufReader::with_capacity(CAPACITY, stored)),
        Representation::Zstd => {
            Box::new(BufReader::with_capacity(CAPACITY, zstd::stream::read::Decoder::new(stored)?))
        }
        Representation::Gzip => {
            Box::new(BufReader::with_capacity(CAPACITY, flate2::read::MultiGzDecoder::new(stored)))
        }
    })
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

/// Drops spare capacity after a long line so one huge record does not pin the worker.
pub(super) fn shrink_line_buffer(buffer: &mut Vec<u8>) {
    if buffer.capacity() > ReadOptions::RETAINED_LINE_CAPACITY {
        buffer.shrink_to(ReadOptions::RETAINED_LINE_CAPACITY);
    }
}

/// Reads one line into `buffer` without its terminator, buffering at most `limit` bytes.
fn read_line(reader: &mut dyn BufRead, buffer: &mut Vec<u8>, limit: usize) -> io::Result<Line> {
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
                if buffer.len().saturating_add(newline) > limit {
                    oversized = true;
                    buffer.clear();
                } else if let Some(line) = available.get(..newline) {
                    buffer.extend_from_slice(line);
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
            if buffer.len().saturating_add(taken) > limit {
                oversized = true;
                buffer.clear();
            } else {
                buffer.extend_from_slice(available);
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

#[expect(
    clippy::too_many_arguments,
    reason = "one call site; splitting it would only move the arguments"
)]
fn detect_changes(
    path: &Path,
    representation: Representation,
    snapshot: &FileIdentity,
    snapshot_len: u64,
    snapshot_modified: Option<SystemTime>,
    fingerprint: Option<Fingerprint>,
    options: &ReadOptions,
    hooks: &mut dyn ScanHooks,
    changes: &mut Vec<SourceChange>,
) {
    let mut attempts = 0;
    let metadata = loop {
        match std::fs::metadata(path) {
            Ok(metadata) => break Some(metadata),
            Err(error)
                if error.kind() == io::ErrorKind::NotFound && attempts < options.absent_retries =>
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
    let Some(metadata) = metadata else {
        changes.push(SourceChange::Vanished);
        return;
    };
    if attempts > 0
        && !changes.iter().any(|change| matches!(change, SourceChange::BrieflyAbsent { .. }))
    {
        changes.push(SourceChange::BrieflyAbsent { attempts });
    }
    if device_of(&metadata) != snapshot.device || inode_of(&metadata) != snapshot.inode {
        changes.push(SourceChange::Replaced);
        return;
    }
    let observed_len = metadata.len();
    if observed_len < snapshot_len {
        changes.push(SourceChange::Truncated { snapshot_len, observed_len });
    } else if observed_len > snapshot_len {
        changes.push(SourceChange::GrewBeyondCutoff { observed_len });
    } else if metadata.modified().ok() != snapshot_modified {
        changes.push(SourceChange::ModifiedInPlace);
    }
    if let Some(fingerprint) = fingerprint {
        if first_record_fingerprint(path, representation) != Some(fingerprint) {
            changes.push(SourceChange::FirstRecordChanged);
        }
    }
}

/// Re-reads a file's first complete record to check that the bytes the source ID covers
/// did not change; `None` when the file cannot be read or holds no complete record.
fn first_record_fingerprint(path: &Path, representation: Representation) -> Option<Fingerprint> {
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut reader = decode(file, representation).ok()?;
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match read_line(&mut reader, &mut buffer, ReadOptions::DEFAULT_MAX_RECORD_BYTES).ok()? {
            Line::Complete { .. } => {
                if !buffer.iter().all(u8::is_ascii_whitespace) {
                    return Some(Fingerprint::of(&buffer));
                }
            }
            Line::Eof | Line::Pending { .. } | Line::Oversized { .. } => return None,
        }
    }
}

/// Checks each other discovered file of the source against the file read: a twin whose
/// first record matches is recorded as verified, and one whose first record differs is a
/// different logical source that this scan did not read.
///
/// A twin with no complete first record is neither. It may be gone with the primary,
/// empty, or still being written by a compressor, which writes under the final name; it
/// holds no record that the file read lacks.
fn twin_identities(
    files: &LogicalSource,
    read: &Path,
    fingerprint: Option<Fingerprint>,
    spec: &SourceSpec<'_>,
    failures: &mut Vec<CoverageFailure>,
) -> Vec<FileIdentity> {
    let mut identities = Vec::new();
    for (twin, representation) in files.files().filter(|(path, _)| *path != read) {
        let Some(theirs) = first_record_fingerprint(twin, representation) else { continue };
        if fingerprint != Some(theirs) {
            failures.push(CoverageFailure::TwinFingerprintMismatch {
                path: twin.to_owned(),
                locator: spec.locator.to_owned(),
            });
            continue;
        }
        let metadata = std::fs::metadata(twin).ok();
        identities.push(FileIdentity {
            path: twin.to_owned(),
            device: metadata.as_ref().and_then(device_of_opt),
            inode: metadata.as_ref().and_then(inode_of_opt),
        });
    }
    identities
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

#[cfg(test)]
mod tests;
