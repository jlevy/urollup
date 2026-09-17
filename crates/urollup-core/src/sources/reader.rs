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
//! - **Oversized records.** A record over [`ReadOptions::max_record_bytes`] is never
//!   silently skipped: it is counted and recorded as [`CoverageFailure::Oversized`], and
//!   the scan continues at the next record without buffering it.
//! - **Compression.** `.jsonl` and `.jsonl.zst` decode to the same bytes, so evidence
//!   offsets are decoded offsets and a `.jsonl` file and its `.jsonl.zst` twin are one
//!   logical source with one `src-` ID. A compressed stream that ends inside a frame is
//!   [`CoverageFailure::IncompleteCompressedFrame`], distinct from
//!   [`CoverageFailure::CorruptCompressedData`]. Only structural damage fails the decoder:
//!   a zstd frame carries a content checksum only when its writer asked for one, so a
//!   flipped byte inside a frame usually decodes to damaged text and is counted as
//!   interior corruption instead.
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

/// A cumulative hard limit shared by one or more source scans.
///
/// Decoded bytes are charged after decompression, and records are charged before they
/// are delivered to an adapter. Sharing one value across calls keeps a multi-file
/// ingestion bounded rather than applying the full allowance to every file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadBudget {
    max_decoded_bytes: Option<u64>,
    max_records: Option<u64>,
    decoded_bytes: u64,
    records: u64,
}

impl ReadBudget {
    /// Builds a budget with hard decoded-byte and record-count limits.
    pub const fn new(max_decoded_bytes: u64, max_records: u64) -> Self {
        Self {
            max_decoded_bytes: Some(max_decoded_bytes),
            max_records: Some(max_records),
            decoded_bytes: 0,
            records: 0,
        }
    }

    /// Builds an unlimited budget for callers that manage capacity elsewhere.
    pub const fn unlimited() -> Self {
        Self { max_decoded_bytes: None, max_records: None, decoded_bytes: 0, records: 0 }
    }

    /// Returns the decoded-byte allowance still available, or `None` when unlimited.
    pub fn decoded_bytes_remaining(&self) -> Option<u64> {
        self.max_decoded_bytes.map(|maximum| maximum.saturating_sub(self.decoded_bytes))
    }

    /// Charges decoded bytes consumed outside the primary source scanner.
    ///
    /// Adapters use this for auxiliary inputs, such as sidecars, so every input they
    /// decode participates in the same cumulative ingestion limit.
    pub fn charge_decoded_bytes(&mut self, bytes: u64) -> Result<(), SourceReadError> {
        if self.decoded_bytes_remaining().is_some_and(|remaining| bytes > remaining) {
            return Err(SourceReadError::DecodedByteBudgetExceeded {
                maximum: self.max_decoded_bytes.unwrap_or(0),
            });
        }
        self.decoded_bytes = self.decoded_bytes.saturating_add(bytes);
        Ok(())
    }

    fn charge_record(&mut self) -> Result<(), SourceReadError> {
        if let Some(maximum) = self.max_records {
            if self.records >= maximum {
                return Err(SourceReadError::RecordBudgetExceeded { maximum });
            }
        }
        self.records = self.records.saturating_add(1);
        Ok(())
    }
}

impl Default for ReadBudget {
    fn default() -> Self {
        Self::unlimited()
    }
}

/// What the visitor did with a record, counted per source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordDisposition {
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

/// The files of one logical source: a plain file, a compressed file, or both while a
/// compression or resume is in flight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalSource {
    /// The `.jsonl` file, when it exists.
    pub plain: Option<PathBuf>,
    /// The `.jsonl.zst` file, when it exists.
    pub compressed: Option<PathBuf>,
}

impl LogicalSource {
    /// The representation to read: the plain file hides its compressed twin, as Codex's
    /// own discovery does, because a resume decompresses before appending.
    pub fn primary(&self) -> Option<(&Path, Representation)> {
        match (&self.plain, &self.compressed) {
            (Some(plain), _) => Some((plain.as_path(), Representation::Plain)),
            (None, Some(compressed)) => Some((compressed.as_path(), Representation::Zstd)),
            (None, None) => None,
        }
    }

    /// The other representation, when both exist.
    pub fn twin(&self) -> Option<&Path> {
        match (&self.plain, &self.compressed) {
            (Some(_), Some(compressed)) => Some(compressed.as_path()),
            _ => None,
        }
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
    /// The logical source names no file.
    #[error("logical source has neither a plain nor a compressed file")]
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
    /// Decompressed input exceeded the cumulative ingestion allowance.
    #[error("decoded source input exceeds the ingestion limit of {maximum} bytes")]
    DecodedByteBudgetExceeded {
        /// The maximum decoded bytes allowed across the ingestion.
        maximum: u64,
    },
    /// Source records exceeded the cumulative ingestion allowance.
    #[error("source records exceed the ingestion limit of {maximum}")]
    RecordBudgetExceeded {
        /// The maximum records allowed across the ingestion.
        maximum: u64,
    },
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

/// Reads one logical source under a cumulative decoded-byte and record-count budget.
pub fn read_source_with_budget<F>(
    spec: &SourceSpec<'_>,
    files: &LogicalSource,
    options: &ReadOptions,
    budget: &mut ReadBudget,
    visit: F,
) -> Result<ManifestEntry, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    read_source_with_hooks_and_budget(spec, files, options, budget, visit, &mut NoHooks)
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
    visit: F,
    hooks: &mut dyn ScanHooks,
) -> Result<ManifestEntry, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    let mut budget = ReadBudget::unlimited();
    read_source_with_hooks_and_budget(spec, files, options, &mut budget, visit, hooks)
}

fn read_source_with_hooks_and_budget<F>(
    spec: &SourceSpec<'_>,
    files: &LogicalSource,
    options: &ReadOptions,
    budget: &mut ReadBudget,
    mut visit: F,
    hooks: &mut dyn ScanHooks,
) -> Result<ManifestEntry, SourceReadError>
where
    F: FnMut(&RawRecord<'_>) -> RecordDisposition,
{
    let Some((path, representation)) = files.primary() else {
        return Err(SourceReadError::NoFile);
    };
    let mut changes = Vec::new();
    let opened = open_with_retry(path, options, hooks, &mut changes)?;
    let captured_at = SystemTime::now();
    // The open file's own metadata fixes the extent, so a replacement of the path after
    // this point cannot change what this snapshot covers.
    let metadata = opened
        .metadata()
        .map_err(|source| SourceReadError::Open { path: path.to_owned(), source })?;
    hooks.after_open(path);

    let snapshot_len = metadata.len();
    let file_identity = FileIdentity {
        path: path.to_owned(),
        device: device_of(&metadata),
        inode: inode_of(&metadata),
    };
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
    scan.run(&mut reader, spec, options, budget, &mut visit)?;
    hooks.after_scan(path);

    let twin = twin_identity(files, spec, options, &mut scan.failures);
    detect_changes(
        path,
        &file_identity,
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
        file: file_identity,
        representation,
        twin,
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
        budget: &mut ReadBudget,
        visit: &mut F,
    ) -> Result<(), SourceReadError>
    where
        F: FnMut(&RawRecord<'_>) -> RecordDisposition,
    {
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            let line = match read_line(
                reader,
                &mut buffer,
                options.max_record_bytes,
                budget.decoded_bytes_remaining(),
            ) {
                Ok(line) => line,
                Err(error) => {
                    self.failures.push(read_failure(&error, self.offset));
                    return Ok(());
                }
            };
            match line {
                Line::Eof => return Ok(()),
                Line::BudgetExceeded => {
                    return Err(SourceReadError::DecodedByteBudgetExceeded {
                        maximum: budget.max_decoded_bytes.unwrap_or(0),
                    });
                }
                Line::Pending { length, oversized } => {
                    budget.charge_decoded_bytes(length)?;
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
                    budget.charge_decoded_bytes(length.saturating_add(1))?;
                    self.counters.oversized = self.counters.oversized.saturating_add(1);
                    self.failures.push(CoverageFailure::Oversized { offset: self.offset, length });
                    self.advance(length);
                }
                Line::Complete { length } => {
                    budget.charge_decoded_bytes(length.saturating_add(1))?;
                    if buffer.iter().all(u8::is_ascii_whitespace) {
                        self.counters.blank_lines = self.counters.blank_lines.saturating_add(1);
                        self.advance(length);
                        continue;
                    }
                    budget.charge_record()?;
                    if self.fingerprint.is_none() {
                        let fingerprint = Fingerprint::of(&buffer);
                        self.fingerprint = Some(fingerprint);
                        self.source = Some(source_identity(spec, fingerprint)?);
                    }
                    let Some(source) = self.source.as_ref().map(|stored| stored.id.clone()) else {
                        return Ok(());
                    };
                    let evidence = EvidenceRef { source, offset: self.offset, length };
                    let disposition = visit(&RawRecord { evidence: &evidence, bytes: &buffer });
                    self.counters.records = self.counters.records.saturating_add(1);
                    match disposition {
                        RecordDisposition::Decoded => {
                            self.counters.decoded = self.counters.decoded.saturating_add(1);
                        }
                        RecordDisposition::Skipped => {
                            self.counters.skipped = self.counters.skipped.saturating_add(1);
                        }
                        RecordDisposition::Malformed => {
                            self.counters.malformed = self.counters.malformed.saturating_add(1);
                            if self.first_malformed.is_none() {
                                self.first_malformed = Some(evidence.clone());
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

fn reader_for(file: File, extent: u64, representation: Representation) -> Box<dyn BufRead> {
    let extent = file.take(extent);
    match representation {
        Representation::Plain => Box::new(BufReader::with_capacity(128 * 1024, extent)),
        Representation::Zstd => match zstd::stream::read::Decoder::new(extent) {
            Ok(decoder) => Box::new(BufReader::with_capacity(128 * 1024, decoder)),
            // A decoder is only built here; a failure is a corrupt or empty stream, which
            // the scan reports as a read failure at offset 0.
            Err(error) => Box::new(FailingReader(Some(error))),
        },
    }
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

fn read_failure(error: &io::Error, offset: u64) -> CoverageFailure {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        CoverageFailure::IncompleteCompressedFrame { decoded_offset: offset }
    } else if error.kind() == io::ErrorKind::Other {
        CoverageFailure::CorruptCompressedData {
            decoded_offset: offset,
            message: error.to_string(),
        }
    } else {
        CoverageFailure::ReadError { decoded_offset: offset, kind: error.kind() }
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
    /// Reading another decoded byte would exceed the shared ingestion budget.
    BudgetExceeded,
}

/// Reads one line into `buffer` without its terminator, buffering at most `limit` bytes.
fn read_line(
    reader: &mut dyn BufRead,
    buffer: &mut Vec<u8>,
    limit: usize,
    decoded_remaining: Option<u64>,
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
            let decoded = length.saturating_add(newline as u64).saturating_add(1);
            if decoded_remaining.is_some_and(|remaining| decoded > remaining) {
                return Ok(Line::BudgetExceeded);
            }
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
        let decoded = length.saturating_add(taken as u64);
        if decoded_remaining.is_some_and(|remaining| decoded > remaining) {
            return Ok(Line::BudgetExceeded);
        }
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

fn open_with_retry(
    path: &Path,
    options: &ReadOptions,
    hooks: &mut dyn ScanHooks,
    changes: &mut Vec<SourceChange>,
) -> Result<File, SourceReadError> {
    let mut attempts = 0;
    loop {
        match File::open(path) {
            Ok(file) => {
                if attempts > 0 {
                    changes.push(SourceChange::BrieflyAbsent { attempts });
                }
                return Ok(file);
            }
            Err(error)
                if error.kind() == io::ErrorKind::NotFound && attempts < options.absent_retries =>
            {
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

#[expect(
    clippy::too_many_arguments,
    reason = "one call site; splitting it would only move the arguments"
)]
fn detect_changes(
    path: &Path,
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
        if first_record_fingerprint(path) != Some(fingerprint) {
            changes.push(SourceChange::FirstRecordChanged);
        }
    }
}

/// Re-reads a file's first complete record to check that the bytes the source ID covers
/// did not change; `None` when the file cannot be read or holds no complete record.
fn first_record_fingerprint(path: &Path) -> Option<Fingerprint> {
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut reader: Box<dyn BufRead> =
        if path.extension().is_some_and(|extension| extension == "zst") {
            Box::new(BufReader::new(zstd::stream::read::Decoder::new(file).ok()?))
        } else {
            Box::new(BufReader::new(file))
        };
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match read_line(&mut reader, &mut buffer, ReadOptions::DEFAULT_MAX_RECORD_BYTES, None)
            .ok()?
        {
            Line::Complete { .. } => {
                if !buffer.iter().all(u8::is_ascii_whitespace) {
                    return Some(Fingerprint::of(&buffer));
                }
            }
            Line::Eof | Line::Pending { .. } | Line::Oversized { .. } | Line::BudgetExceeded => {
                return None;
            }
        }
    }
}

/// Checks a compressed twin against the primary file and records its identity, or reports
/// that it is a different logical source that this scan did not read.
fn twin_identity(
    files: &LogicalSource,
    spec: &SourceSpec<'_>,
    _options: &ReadOptions,
    failures: &mut Vec<CoverageFailure>,
) -> Option<FileIdentity> {
    let twin = files.twin()?;
    let primary = files.primary()?.0;
    let (primary_fingerprint, twin_fingerprint) =
        (first_record_fingerprint(primary), first_record_fingerprint(twin));
    if primary_fingerprint.is_some() && primary_fingerprint != twin_fingerprint {
        failures.push(CoverageFailure::TwinFingerprintMismatch {
            path: twin.to_owned(),
            locator: spec.locator.to_owned(),
        });
        return None;
    }
    let metadata = std::fs::metadata(twin).ok();
    Some(FileIdentity {
        path: twin.to_owned(),
        device: metadata.as_ref().and_then(device_of_opt),
        inode: metadata.as_ref().and_then(inode_of_opt),
    })
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
