//! Snapshot manifests: what each run read from each source file (design §2.2).
//!
//! Files are not snapshotted atomically together, so each [`ManifestEntry`] records its
//! own cutoff: the file's identity and length when it was opened, the decoded extent of
//! complete records read, any pending unfinished tail, and when the snapshot was taken.
//! [`SnapshotManifest::cutoff_skew`] reports the spread of those times across files.
//!
//! A source's `src-` ID derives from its source environment, dialect, locator and the
//! [`Fingerprint`] of its first complete record, so appended records keep the ID and a
//! rewrite of the first record gets a new one. The locator is root-relative unless the
//! dialect declares a stable one ([`SOURCE_STABLE_LOCATOR`]).

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::ledger::identity::{IdPrefix, StoredIdentity, crockford_base32_128, sha256_128};
use crate::ledger::scope::{ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec};
use crate::sources::evidence::EvidenceRef;

const SOURCE_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("environment", ComponentRole::Environment),
    ComponentSlot::required("dialect", ComponentRole::Dialect),
    ComponentSlot::required("locator", ComponentRole::Locator),
    ComponentSlot::required("first_record_digest", ComponentRole::Digest),
];

/// The `src-` key kind for a dialect-declared stable locator, such as a Codex rollout's
/// thread and rollout IDs, which survive archiving and compression.
pub const SOURCE_STABLE_LOCATOR: KeySpec = KeySpec {
    prefix: IdPrefix::Source,
    kind: "stable-locator",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Source,
    slots: SOURCE_SLOTS,
};

/// The `src-` key kind for a root-relative locator, with `/` separators and any
/// compression suffix removed, so a `.jsonl` file and its `.jsonl.zst` twin share it.
pub const SOURCE_ROOT_RELATIVE: KeySpec = KeySpec {
    prefix: IdPrefix::Source,
    kind: "root-relative",
    precedence: 1,
    basis: IdentityBasis::Native,
    scope: IdScope::Source,
    slots: SOURCE_SLOTS,
};

/// The first 128 bits of SHA-256 over a record's bytes, excluding its line terminator.
///
/// A fingerprint identifies bytes, not necessarily a logical session.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fingerprint([u8; 16]);

impl Fingerprint {
    /// The fingerprint of `record`.
    pub fn of(record: &[u8]) -> Self {
        Self(sha256_128(record))
    }

    /// The 26-digit lowercase Crockford base32 form used in keys and reports.
    pub fn to_base32(&self) -> String {
        crockford_base32_128(&self.0)
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base32())
    }
}

/// How a source's bytes are stored.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Representation {
    /// Plain `.jsonl`.
    Plain,
    /// zstd-compressed `.jsonl.zst`, possibly in several frames.
    Zstd,
}

/// A file's identity when it was opened.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileIdentity {
    /// The path read.
    pub path: PathBuf,
    /// The device ID, where the platform reports one (Unix).
    pub device: Option<u64>,
    /// The inode number, where the platform reports one (Unix).
    pub inode: Option<u64>,
}

/// The unfinished bytes after the last complete record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PendingTail {
    /// Decoded offset of the first pending byte.
    pub offset: u64,
    /// Pending bytes.
    pub length: u64,
}

/// Where a source's snapshot ends.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cutoff {
    /// Decoded bytes through the terminator of the last complete record.
    pub complete_through: u64,
    /// The unfinished last line, recorded as pending rather than as corruption.
    pub pending_tail: Option<PendingTail>,
    /// When the file was opened for this snapshot.
    pub captured_at: SystemTime,
}

/// Per-source line counts. Every line lands in exactly one of `blank_lines`, `oversized`
/// or `records`, and every record in exactly one of the four dispositions.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordCounters {
    /// Complete records passed to the decoder.
    pub records: u64,
    /// Complete lines holding only whitespace.
    pub blank_lines: u64,
    /// Complete lines over the record size limit, each also a coverage failure.
    pub oversized: u64,
    /// Records the decoder used.
    pub decoded: u64,
    /// Records the decoder skipped as irrelevant, including prefilter misses.
    pub skipped: u64,
    /// Complete records that are not valid JSON: interior corruption.
    pub malformed: u64,
    /// Valid JSON records the dialect could not interpret.
    pub unparsable: u64,
}

/// Data a snapshot could not read, never silently skipped.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoverageFailure {
    /// A complete record over the size limit.
    Oversized {
        /// Decoded offset of the record.
        offset: u64,
        /// Its length, excluding the terminator.
        length: u64,
    },
    /// Compressed data that does not decode, at or after a decoded offset.
    CorruptCompressedData {
        /// Decoded bytes produced before the failure.
        decoded_offset: u64,
        /// The decoder's message.
        message: String,
    },
    /// A compressed stream that ends inside a frame; its decoded tail is pending.
    IncompleteCompressedFrame {
        /// Decoded bytes produced before the stream ended.
        decoded_offset: u64,
    },
    /// An I/O error ended the scan early.
    ReadError {
        /// Decoded bytes read before the error.
        decoded_offset: u64,
        /// The error kind.
        kind: io::ErrorKind,
    },
}

/// A change to a source detected around or during its scan.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceChange {
    /// The path was absent when opened or re-checked and reappeared after retries.
    BrieflyAbsent {
        /// Retries needed before the path reappeared.
        attempts: u32,
    },
    /// The path was absent after the scan and did not reappear.
    Vanished,
    /// The path names a different file after the scan.
    Replaced,
    /// The file became shorter than its snapshot length.
    Truncated {
        /// Length at snapshot.
        snapshot_len: u64,
        /// Length afterwards.
        observed_len: u64,
    },
    /// The file kept its length but its modification time changed.
    ModifiedInPlace,
    /// The first complete record changed, so the bytes the source ID covers were rewritten.
    FirstRecordChanged,
    /// Records were appended after the cutoff; they belong to a later snapshot.
    GrewBeyondCutoff {
        /// Length afterwards.
        observed_len: u64,
    },
}

impl SourceChange {
    /// Whether the change can make the snapshot's records disagree with the file, as
    /// opposed to an append past the cutoff.
    pub const fn affects_snapshot(&self) -> bool {
        match self {
            Self::GrewBeyondCutoff { .. } | Self::BrieflyAbsent { .. } => false,
            Self::Vanished
            | Self::Replaced
            | Self::Truncated { .. }
            | Self::ModifiedInPlace
            | Self::FirstRecordChanged => true,
        }
    }
}

/// One logical source's snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ManifestEntry {
    /// The `src-` ID and key; `None` when the source has no complete record.
    pub source: Option<StoredIdentity>,
    /// The source environment registry token.
    pub environment: String,
    /// The dialect registry token.
    pub dialect: String,
    /// The locator digested into the source key.
    pub locator: String,
    /// The file read.
    pub file: FileIdentity,
    /// How it is stored.
    pub representation: Representation,
    /// The other representation of this logical source, when both exist.
    pub twin: Option<FileIdentity>,
    /// On-disk length when opened.
    pub file_len: u64,
    /// Modification time when opened.
    pub modified: Option<SystemTime>,
    /// Fingerprint of the first complete record.
    pub fingerprint: Option<Fingerprint>,
    /// The snapshot's cutoff.
    pub cutoff: Cutoff,
    /// Line and record counts.
    pub counters: RecordCounters,
    /// The first malformed record, for reports.
    pub first_malformed: Option<EvidenceRef>,
    /// Data that could not be read.
    pub failures: Vec<CoverageFailure>,
    /// Changes detected around the scan.
    pub changes: Vec<SourceChange>,
}

impl ManifestEntry {
    /// Whether the snapshot read every byte of its extent as complete records, with no
    /// pending tail, corruption, failure or change that affects the snapshot.
    pub fn is_complete(&self) -> bool {
        self.cutoff.pending_tail.is_none()
            && self.counters.malformed == 0
            && self.failures.is_empty()
            && !self.changes.iter().any(SourceChange::affects_snapshot)
    }
}

/// A symlink discovery did not follow.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SkippedLink {
    /// The link's path.
    pub path: PathBuf,
    /// Why it was skipped.
    pub reason: SkippedLinkReason,
}

/// Why a symlink was not followed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SkippedLinkReason {
    /// Its target lies outside every declared root.
    OutsideRoots,
    /// Its target does not exist.
    Broken,
    /// Its target is a directory already walked, which would repeat or loop.
    AlreadyVisited,
}

/// Every source one run read, with the links it did not follow.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct SnapshotManifest {
    /// Source entries, in the order they were added.
    pub entries: Vec<ManifestEntry>,
    /// Symlinks discovery skipped.
    pub skipped_links: Vec<SkippedLink>,
}

impl SnapshotManifest {
    /// The spread between the earliest and latest per-source cutoff; `None` for an empty
    /// manifest.
    pub fn cutoff_skew(&self) -> Option<Duration> {
        let times = self.entries.iter().map(|entry| entry.cutoff.captured_at);
        let earliest = times.clone().min()?;
        let latest = times.max()?;
        Some(latest.duration_since(earliest).unwrap_or(Duration::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::{SOURCE_ROOT_RELATIVE, SOURCE_STABLE_LOCATOR};
    use crate::ledger::scope::validate_specs;

    #[test]
    fn source_key_kinds_are_valid_and_ranked() {
        validate_specs(&[SOURCE_STABLE_LOCATOR, SOURCE_ROOT_RELATIVE]).unwrap();
    }

    #[test]
    fn skew_is_the_spread_of_cutoffs() {
        let mut manifest = super::SnapshotManifest::default();
        assert_eq!(manifest.cutoff_skew(), None);
        let base = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        for offset in [5, 0, 2] {
            let mut entry = super::tests_support::entry();
            entry.cutoff.captured_at = base + Duration::from_millis(offset);
            manifest.entries.push(entry);
        }
        assert_eq!(manifest.cutoff_skew(), Some(Duration::from_millis(5)));
    }
}

#[cfg(test)]
pub(crate) mod tests_support {
    use std::path::PathBuf;
    use std::time::SystemTime;

    use super::{Cutoff, FileIdentity, ManifestEntry, RecordCounters, Representation};

    /// A minimal entry for tests that only vary a few fields.
    pub(crate) fn entry() -> ManifestEntry {
        ManifestEntry {
            source: None,
            environment: "local".to_owned(),
            dialect: "test".to_owned(),
            locator: "a.jsonl".to_owned(),
            file: FileIdentity { path: PathBuf::from("a.jsonl"), device: None, inode: None },
            representation: Representation::Plain,
            twin: None,
            file_len: 0,
            modified: None,
            fingerprint: None,
            cutoff: Cutoff {
                complete_through: 0,
                pending_tail: None,
                captured_at: SystemTime::UNIX_EPOCH,
            },
            counters: RecordCounters::default(),
            first_malformed: None,
            failures: Vec::new(),
            changes: Vec::new(),
        }
    }
}
