//! Codex rollout adapter (`codex-rollout`).
//!
//! Decoding keeps memory proportional to relevant records, not to log bytes:
//!
//! - A line the byte prefilter rules out is only validated. Any other line is read by a
//!   typed first pass, which borrows its strings and builds no JSON document. A
//!   `rate_limits` object is reduced to compact native JSON while it is read.
//! - A decoded record is a compact row that allocates nothing of its own: the strings it
//!   names are interned in its rollout's string table, its usage counts are packed into
//!   its rollout's count buffer, and consecutive identical rate-limit snapshots share one
//!   value.
//! - Session metadata is summarized per rollout while it decodes, and each rollout's
//!   records are freed as soon as its observations are built.

mod line;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{BuildHasher, RandomState};
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use self::line::{
    DecodedLimits, EventType, HistoryBoundary, Line, Payload, RecordType, UsageFields,
};
use super::{AdapterError, Ingested};
use crate::ledger::admission::MemoryAdmission;
use crate::ledger::capacity::ObservationCapacity;
use crate::ledger::counters::{CounterEvent, RunningTotal};
use crate::ledger::coverage::{CoverageGap, UnobservedReason};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode, SAMPLE_EVIDENCE_LIMIT};
use crate::ledger::entities::{
    Basis, CompactTimestamp, Confidence, ModelBasis, ModelName, ProviderLimitObservation,
    Relationship, RelationshipKind, SourceArtifact, SourceCapability, Thread,
};
use crate::ledger::identity::{AnalyticalId, IdPrefix, KeyComponent, StoredIdentity, sha256_128};
use crate::ledger::names::Name;
use crate::ledger::reconcile::{
    LatestRevision, ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation,
    reconcile_with_capacity,
};
use crate::ledger::scope::{ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec};
use crate::ledger::tokens::{InputSemantics, NativeInput, TokenMeasures, normalize_input};
use crate::selection::{Agent, agent_thread_identity};
use crate::sources::decode::{parse_timestamp, validate_record};
use crate::sources::evidence::{EvidenceRef, SourceTable};
use crate::sources::manifest::{ManifestEntry, Representation, SnapshotManifest};
use crate::sources::parallel::{default_workers, source_weight, try_read_in_parallel};
use crate::sources::reader::{RawRecord, ReadOptions, RecordDisposition, SourceSpec, read_source};
use crate::sources::roots::{DiscoveredSource, Discovery, discover};

const DIALECT: &str = "codex-rollout";
const PROVIDER_NAMESPACE: &str = "openai";

const PROVIDER_RESPONSE_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("provider", ComponentRole::Namespace),
    ComponentSlot::required("response_id", ComponentRole::NativeId),
];

const RESPONSE_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "provider-response",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Provider,
    slots: PROVIDER_RESPONSE_SLOTS,
};

const COUNTER_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("thread", ComponentRole::Parent),
    ComponentSlot::required("cumulative_usage", ComponentRole::Digest),
];

const COUNTER_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "thread-counter-digest",
    precedence: 1,
    basis: IdentityBasis::Fallback,
    scope: IdScope::Thread,
    slots: COUNTER_SLOTS,
};

/// The largest a decoded record may be.
///
/// One record is held for every relevant line of every rollout until its rollout's
/// observations are built, so this size bounds that phase: the 508,000 records of a 12 GB
/// Codex history take 39 MiB inline, with no allocation of their own. The layout is
/// 76 bytes padded to 80: 16 of evidence position, an 8-byte count of skipped lines, a
/// 16-byte optional ordinal, a 12-byte optional timestamp, and a 24-byte kind whose
/// largest variants are a usage record (three 4-byte symbols and a 2-byte count mask,
/// behind an option), a turn context (a symbol and two 8-byte names) and a token count (a
/// count mask, an 8-byte shared rate-limit pointer and a 1-byte role).
const _: () =
    assert!(size_of::<ParsedRecord>() <= 80, "a decoded Codex record outgrew its size budget");

/// The bytes of one decoded record, which admission charges while its rollout waits to be
/// observed (`ledger::admission::model::decoded_record`).
pub(crate) const DECODED_RECORD_BYTES: u64 = size_of::<ParsedRecord>() as u64;

/// Everything one rollout contributes before normalization.
struct ParsedSource {
    /// The source ID every record of the rollout shares; `None` only without records.
    id: Option<AnalyticalId>,
    /// The thread the rollout's file name records.
    file_thread: Sym,
    /// The table that this rollout's symbols index.
    strings: Strings,
    /// The usage counts of this rollout's records, packed in record order.
    counts: Vec<u8>,
    records: Vec<ParsedRecord>,
    metas: SourceMetas,
    /// Lines skipped after the last record.
    trailing_skipped: u64,
    /// Which `token_count` events the counter rules account for.
    counter_scope: CounterScope,
    /// Records that can become request or limit observations: usage, compacted, and
    /// token-count lines. Session meta and turn context stay in `records` but do not
    /// reserve observation slots.
    observation_slots: usize,
    /// Turn-ID digests from this rollout when it is a root, collected while decoding so
    /// normalization does not walk every record again to build [`KnownTurns`].
    root_turns: Vec<TurnDigest>,
}

/// One rollout after decode: usage-bearing files are observed on the worker so their
/// records never sit in the join result.
enum DecodedRollout {
    Observed(ObservedSource),
    Pending(ParsedSource),
}

/// A rollout whose records were observed and dropped on the decoding worker.
struct ObservedSource {
    id: Option<AnalyticalId>,
    file_thread: String,
    metas: SourceMetas,
    root_turns: Vec<TurnDigest>,
    observations: Vec<RequestObservation>,
    limit_observations: Vec<ProviderLimitObservation>,
    diagnostics: Vec<Diagnostic>,
    gaps: Vec<CoverageGap>,
    copied_regions: u64,
}

/// Observations, limits, diagnostics and coverage gaps from one rollout.
struct SourceObserve {
    observations: Vec<RequestObservation>,
    limit_observations: Vec<ProviderLimitObservation>,
    diagnostics: Vec<Diagnostic>,
    gaps: Vec<CoverageGap>,
    copied_regions: u64,
}

/// The accounting fields of one relevant rollout record.
///
/// Its evidence source is its rollout's source ID, and the strings and counts it names
/// are in its rollout's tables.
struct ParsedRecord {
    offset: u64,
    length: u64,
    /// Lines skipped since the previous record.
    skipped_before: u64,
    ordinal: Option<u64>,
    timestamp: Option<CompactTimestamp>,
    kind: RecordKind,
}

/// The relevant record kinds and the fields the adapter reads from each.
enum RecordKind {
    SessionMeta {
        id: Option<Sym>,
    },
    TurnContext {
        turn_id: Option<Sym>,
        model: Option<Name>,
        effort: Option<Name>,
    },
    UsageRecord(UsageRecord),
    /// A compaction, with the latest usage record it copies when the field is present.
    Compacted(Option<UsageRecord>),
    /// A token count, whose total usage is in count slot 0 and last usage in slot 1.
    TokenCount {
        usage: UsageMask,
        limits: Option<Arc<RateLimits>>,
        /// How the accounting treats it, decided when the rollout finishes decoding.
        role: CounterRole,
    },
    ThreadSettingsApplied {
        thread_id: Option<Sym>,
    },
}

impl ParsedRecord {
    fn evidence(&self, source: u32) -> EvidenceRef {
        EvidenceRef::new(source, self.offset, self.length)
    }
}

/// A `token_usage_record` payload, or the latest one a `compacted` record copies, whose
/// usage is in count slot 0.
#[derive(Clone, Copy)]
struct UsageRecord {
    thread_id: Option<Sym>,
    response_id: Option<Sym>,
    /// The turn whose context applies: the record's `turn_id`, else its `root_turn_id`.
    /// A subagent's records name its parent's turn as their root, so only `turn_id`
    /// matches the subagent's own `turn_context`.
    turn: Option<Sym>,
    usage: UsageMask,
}

impl UsageRecord {
    fn payload<'a>(&self, strings: &'a Strings, usage: Option<CodexUsage>) -> UsagePayload<'a> {
        UsagePayload {
            thread_id: self.thread_id.map(|thread| strings.resolve(thread)),
            response_id: self.response_id.map(|response| strings.resolve(response)),
            usage,
            turn: self.turn,
        }
    }
}

/// A usage record with its strings and counts read back.
struct UsagePayload<'a> {
    thread_id: Option<&'a str>,
    response_id: Option<&'a str>,
    usage: Option<CodexUsage>,
    turn: Option<Sym>,
}

/// Where a record is and when it was written, as the observations built from it cite it.
struct RecordView {
    evidence: EvidenceRef,
    timestamp: Option<CompactTimestamp>,
}

/// The session metadata a rollout records for its own thread.
#[derive(Clone, Default)]
struct SessionMeta {
    source: Option<String>,
    thread_source: Option<String>,
    /// The final component of the recorded working directory, never the path.
    project: Option<String>,
    parent_thread_id: Option<String>,
    forked_from_id: Option<String>,
    subagent_history_start_ordinal: HistoryBoundary,
}

impl SessionMeta {
    fn parent(&self) -> Option<&str> {
        self.parent_thread_id.as_deref().or(self.forked_from_id.as_deref())
    }
}

/// What normalization reads from a rollout's `session_meta` records, summarized while
/// the rollout decodes.
#[derive(Default)]
struct SourceMetas {
    /// The first `cli_version` any session meta reports.
    cli_version: Option<String>,
    /// Whether the first session meta names the rollout's own thread with neither a parent
    /// nor a fork origin; `None` before any session meta.
    root: Option<bool>,
    /// The first session meta that names the rollout's own thread, with its evidence.
    own: Option<Box<(SessionMeta, EvidenceRef)>>,
    /// Whether any session meta names another thread.
    foreign: bool,
    /// Whether any session meta is not the rollout's own, names a top-level parent or fork
    /// origin, or declares `subagent_history_start_ordinal`: the rollout can hold another
    /// thread's history.
    lineage: bool,
    /// Whether any session meta names a parent only in
    /// `source.subagent.thread_spawn.parent_thread_id`, as an old-format subagent written
    /// before Codex's top-level `parent_thread_id` does. Such a rollout never decides a
    /// child's turns, but it is not lineage: without a header or boundary it holds no copy,
    /// so its own counters keep the root rules.
    spawn_parent: bool,
}

/// Native Codex usage counters, each `None` when missing, null or not a count.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CodexUsage {
    input: Option<u64>,
    cached_input: Option<u64>,
    cache_write_input: Option<u64>,
    output: Option<u64>,
    reasoning_output: Option<u64>,
    total: Option<u64>,
}

/// The number of counters in a [`CodexUsage`].
const USAGE_COUNTS: usize = 6;

impl CodexUsage {
    fn counts(self) -> [Option<u64>; USAGE_COUNTS] {
        [
            self.input,
            self.cached_input,
            self.cache_write_input,
            self.output,
            self.reasoning_output,
            self.total,
        ]
    }

    fn from_counts(counts: [Option<u64>; USAGE_COUNTS]) -> Self {
        let [input, cached_input, cache_write_input, output, reasoning_output, total] = counts;
        Self { input, cached_input, cache_write_input, output, reasoning_output, total }
    }
}

/// Which usage objects a record carries, in numbered slots, and which of their counts are
/// present: each slot has a presence bit followed by one bit per count, in
/// [`CodexUsage::counts`] order. The present counts are packed into the rollout's count
/// buffer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct UsageMask(u16);

impl UsageMask {
    const SLOT_BITS: u32 = 7;

    fn has(self, bit: u32) -> bool {
        self.0 & (1 << bit) != 0
    }
}

/// Packs a usage object into `slot`, writing each present count as an LEB128 varint.
fn pack_usage(counts: &mut Vec<u8>, mask: &mut UsageMask, slot: u32, usage: Option<CodexUsage>) {
    let Some(usage) = usage else { return };
    let base = slot * UsageMask::SLOT_BITS;
    mask.0 |= 1 << base;
    for (bit, count) in (base + 1..).zip(usage.counts()) {
        let Some(mut value) = count else { continue };
        mask.0 |= 1 << bit;
        loop {
            let low = value.to_le_bytes()[0] & 0x7f;
            value >>= 7;
            if value == 0 {
                counts.push(low);
                break;
            }
            counts.push(low | 0x80);
        }
    }
}

/// Reads a rollout's packed counts back, record by record in record order.
struct CountReader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> CountReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    /// The usage objects of the next record, by slot.
    fn record(&mut self, kind: &RecordKind) -> [Option<CodexUsage>; 2] {
        match kind {
            RecordKind::UsageRecord(record) | RecordKind::Compacted(Some(record)) => {
                [self.usage(record.usage, 0), None]
            }
            RecordKind::TokenCount { usage, .. } => [self.usage(*usage, 0), self.usage(*usage, 1)],
            RecordKind::SessionMeta { .. }
            | RecordKind::TurnContext { .. }
            | RecordKind::Compacted(None)
            | RecordKind::ThreadSettingsApplied { .. } => [None, None],
        }
    }

    fn usage(&mut self, mask: UsageMask, slot: u32) -> Option<CodexUsage> {
        let base = slot * UsageMask::SLOT_BITS;
        if !mask.has(base) {
            return None;
        }
        let mut counts = [None; USAGE_COUNTS];
        for (bit, count) in (base + 1..).zip(&mut counts) {
            if mask.has(bit) {
                *count = Some(self.varint());
            }
        }
        Some(CodexUsage::from_counts(counts))
    }

    fn varint(&mut self) -> u64 {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        while let Some(&byte) = self.bytes.get(self.at) {
            self.at = self.at.saturating_add(1);
            value |= u64::from(byte & 0x7f).checked_shl(shift).unwrap_or(0);
            if byte & 0x80 == 0 {
                break;
            }
            shift = shift.saturating_add(7);
        }
        value
    }
}

/// One `rate_limits` object: its limit name, the windows it reports, and its fields as
/// sorted compact JSON. Consecutive identical snapshots in a rollout share one value.
struct RateLimits {
    limit_name: Option<String>,
    windows: Vec<&'static str>,
    native: String,
}

/// A turn's model and reasoning effort.
#[derive(Clone, Copy, Debug, Default)]
struct TurnContext {
    model: Option<Name>,
    effort: Option<Name>,
}

/// The turn contexts of one rollout, by turn ID.
type Turns = HashMap<Sym, TurnContext>;

/// A 128-bit SHA-256 digest of a turn ID, which only joins a child's turns to its parent's.
type TurnDigest = [u8; 16];

/// The turn IDs of each root rollout's thread.
type KnownTurns = HashMap<String, HashSet<TurnDigest>>;

fn turn_digest(turn: &str) -> TurnDigest {
    sha256_128(turn.as_bytes())
}

/// A 128-bit SHA-256 digest of a cumulative total, which only matches a child's copied
/// counter to the parent counter it copies.
type TotalDigest = [u8; 16];

/// A missing count reads as 0: Codex's migration re-serializes a legacy counter with
/// `cache_write_input_tokens: 0`, which the unmigrated original omits.
fn total_digest(total: &CodexUsage) -> TotalDigest {
    let mut bytes = Vec::with_capacity(USAGE_COUNTS * 8);
    for count in total.counts() {
        bytes.extend_from_slice(&count.unwrap_or(0).to_le_bytes());
    }
    sha256_128(&bytes)
}

/// What other rollouts show about the parent whose history a child's prefix may copy.
#[derive(Default)]
struct ParentEvidence {
    /// The turn IDs of each root rollout's thread.
    turns: KnownTurns,
    /// The roots none of whose rollouts holds lineage, the only ones whose turns and totals
    /// can decide a child's prefix: a thread with a parent link, a boundary or another
    /// thread's header may have dropped turns the child copied, as a bounded migration of
    /// an old-format subagent does.
    complete: HashSet<String>,
    /// The cumulative totals of each counter-only root that a child deciding its prefix by
    /// turns names as its parent ([`infers_prefix_turns`]).
    totals: HashMap<String, HashSet<TotalDigest>>,
}

/// The cumulative totals a rollout's counters report.
fn counter_totals(source: &ParsedSource) -> HashSet<TotalDigest> {
    let mut counts = CountReader::new(&source.counts);
    let mut totals = HashSet::new();
    for record in &source.records {
        let [total, _] = counts.record(&record.kind);
        if let (RecordKind::TokenCount { .. }, Some(total)) = (&record.kind, total) {
            totals.insert(total_digest(&total));
        }
    }
    totals
}

fn can_emit_observation(kind: &RecordKind) -> bool {
    matches!(
        kind,
        RecordKind::UsageRecord(_) | RecordKind::Compacted(_) | RecordKind::TokenCount { .. }
    )
}

fn root_turn_digests(
    records: &[ParsedRecord],
    strings: &Strings,
    is_root: bool,
) -> Vec<TurnDigest> {
    if !is_root {
        return Vec::new();
    }
    let mut turns: Vec<TurnDigest> = records
        .iter()
        .filter_map(|record| match record.kind {
            RecordKind::TurnContext { turn_id, .. } => {
                turn_id.map(|turn| turn_digest(strings.resolve(turn)))
            }
            RecordKind::SessionMeta { .. }
            | RecordKind::UsageRecord(_)
            | RecordKind::Compacted(_)
            | RecordKind::TokenCount { .. }
            | RecordKind::ThreadSettingsApplied { .. } => None,
        })
        .collect();
    turns.shrink_to_fit();
    turns
}

/// Whether `thread` is a root thread that never recorded `turn`.
fn is_unknown_turn(known_turns: &KnownTurns, strings: &Strings, thread: Sym, turn: Sym) -> bool {
    known_turns
        .get(strings.resolve(thread))
        .is_some_and(|known| !known.contains(&turn_digest(strings.resolve(turn))))
}

/// An interned string: its position in its rollout's string table, plus one.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Sym(NonZeroU32);

impl Sym {
    fn index(self) -> usize {
        usize::try_from(self.0.get()).map_or(usize::MAX, |position| position.saturating_sub(1))
    }
}

/// The strings one rollout's records name, each stored once in one buffer.
///
/// Symbols from one table compare equal exactly when their strings do.
#[derive(Debug, Default)]
struct Strings {
    text: String,
    /// Where each symbol's text ends; each starts where the previous one ends.
    ends: Vec<usize>,
}

impl Strings {
    fn resolve(&self, sym: Sym) -> &str {
        let index = sym.index();
        let start = index.checked_sub(1).map_or(0, |previous| self.ends[previous]);
        &self.text[start..self.ends[index]]
    }
}

/// A rollout's string table while it decodes, with the index that finds a string's symbol.
#[derive(Default)]
struct Interner {
    strings: Strings,
    hasher: RandomState,
    /// The latest symbol with each text hash.
    latest: HashMap<u64, Sym>,
    /// For each symbol, the previous symbol with the same text hash.
    earlier: Vec<Option<Sym>>,
}

impl Interner {
    fn intern(&mut self, text: &str) -> Sym {
        let hash = self.hasher.hash_one(text);
        let mut candidate = self.latest.get(&hash).copied();
        while let Some(sym) = candidate {
            if self.strings.resolve(sym) == text {
                return sym;
            }
            candidate = self.earlier[sym.index()];
        }
        self.strings.text.push_str(text);
        self.strings.ends.push(self.strings.text.len());
        let sym = u32::try_from(self.strings.ends.len())
            .ok()
            .and_then(NonZeroU32::new)
            .map(Sym)
            .expect("a rollout names fewer than 2^32 distinct strings");
        self.earlier.push(self.latest.insert(hash, sym));
        sym
    }

    fn intern_some(&mut self, text: Option<&str>) -> Option<Sym> {
        text.map(|text| self.intern(text))
    }

    /// The finished table, without its index.
    fn finish(self) -> Strings {
        let mut strings = self.strings;
        strings.text.shrink_to_fit();
        strings.ends.shrink_to_fit();
        strings
    }
}

/// Reads one Codex home, including active, archived and compressed rollouts.
pub fn ingest_root(root: &Path) -> Result<Ingested, AdapterError> {
    ingest_roots(&[root.to_owned()], true)
}

/// Reads Codex homes selected by discovery.
///
/// Missing variable- or flag-selected homes are errors; missing conventional defaults
/// are skipped.
pub fn ingest_roots(roots: &[PathBuf], missing_is_error: bool) -> Result<Ingested, AdapterError> {
    let rollout_roots = rollout_roots(roots);
    let discovery = discover(&rollout_roots);
    ingest_discovery(discovery, missing_is_error)
}

/// Reads a previously discovered set of Codex rollouts.
///
/// Callers that need exact session selection can filter `discovery.sources` before
/// invoking this function, avoiding a second walk and full ingestion of unrelated
/// rollouts while preserving paired plain/compressed representations. Rollouts decode on
/// the [default worker bound](default_workers).
pub fn ingest_discovery(
    discovery: Discovery,
    missing_is_error: bool,
) -> Result<Ingested, AdapterError> {
    ingest_discovery_with_workers(discovery, missing_is_error, default_workers())
}

/// Reads discovered Codex rollouts on at most `workers` threads.
///
/// Each rollout decodes independently, and the results merge in discovery order before
/// normalization, so the result is the same for every worker count. A failure returns
/// the error of the first failing rollout in discovery order, as a sequential read does.
/// Exhausting the shared admission budget instead aborts the invocation with a capacity
/// error, taking precedence over errors from sources interrupted by that refusal.
pub fn ingest_discovery_with_workers(
    discovery: Discovery,
    missing_is_error: bool,
    workers: NonZeroUsize,
) -> Result<Ingested, AdapterError> {
    ingest_discovery_with_capacity(
        discovery,
        missing_is_error,
        workers,
        &ObservationCapacity::default(),
    )
}

/// Reads discovered Codex rollouts with an explicit observation ceiling.
pub fn ingest_discovery_with_capacity(
    discovery: Discovery,
    missing_is_error: bool,
    workers: NonZeroUsize,
    capacity: &ObservationCapacity,
) -> Result<Ingested, AdapterError> {
    if let (true, Some(root)) = (missing_is_error, discovery.missing_roots.first()) {
        return Err(AdapterError::MissingRoot(root.clone()));
    }
    if let Some(unreadable) = discovery.unreadable.first() {
        return Err(AdapterError::UnreadablePath {
            path: unreadable.path.clone(),
            kind: unreadable.kind,
        });
    }

    let admission = MemoryAdmission::unlimited().with_row_ceiling(capacity.clone());
    let thread_ids = Arc::new(thread_ids_from_locators(
        discovery.sources.iter().map(|source| source.locator.as_str()),
    )?);
    let decoded = try_read_in_parallel(
        &discovery.sources,
        workers,
        |source| source_weight(&source.files),
        |source| decode_rollout(source, &thread_ids, &admission),
    );
    if let Some(refusal) = admission.refusal() {
        return Err(refusal.into());
    }
    let decoded = decoded?;
    let (entries, rollouts): (Vec<_>, Vec<_>) = decoded.into_iter().unzip();
    let manifest = SnapshotManifest { entries, skipped_links: discovery.skipped_links };

    normalize(rollouts, manifest, Arc::unwrap_or_clone(thread_ids), capacity)
}

/// Reads one rollout, independently of every other source.
fn decode_source(
    source: &DiscoveredSource,
    admission: &MemoryAdmission,
) -> Result<(ManifestEntry, ParsedSource), AdapterError> {
    let rollout = rollout_name(&source.locator);
    let stable_locator = format!("{}/{}", rollout.thread_id, rollout.rollout_id);
    let spec = SourceSpec {
        environment: "local",
        dialect: DIALECT,
        locator: &stable_locator,
        stable_locator: true,
    };
    let path = source
        .files
        .primary()
        .map_or_else(|| source.root.join(&source.locator), |(path, _)| path.to_owned());
    let mut decoder = SourceDecoder::new(&rollout.thread_id);
    let entry = read_source(&spec, &source.files, &ReadOptions::default(), |raw| {
        decoder.decode_with_admission(raw, Some(admission))
    })
    .map_err(|source| AdapterError::Read { path, source })?;
    let mut parsed = decoder.finish();
    parsed.id = entry.source.as_ref().map(|source| source.id.clone());
    Ok((entry, parsed))
}

/// The quoted type tokens a relevant rollout record must contain: a record is relevant only
/// when its `type`, or an `event_msg` payload's `type`, is one of these strings.
const RELEVANT_TYPE_TOKENS: [&[u8]; 6] = [
    b"\"session_meta\"",
    b"\"turn_context\"",
    b"\"token_usage_record\"",
    b"\"compacted\"",
    b"\"token_count\"",
    b"\"thread_settings_applied\"",
];

/// Whether a line can be a relevant record. A `false` answer is exact for records whose
/// type strings are written without escapes, as Codex writes them.
fn may_be_relevant(bytes: &[u8]) -> bool {
    RELEVANT_TYPE_TOKENS.iter().any(|token| memchr::memmem::find(bytes, token).is_some())
}

/// The state of one rollout while its lines decode.
struct SourceDecoder {
    id: Option<AnalyticalId>,
    file_thread: Sym,
    interner: Interner,
    counts: Vec<u8>,
    records: Vec<ParsedRecord>,
    metas: SourceMetas,
    /// Lines skipped since the last record.
    skipped: u64,
    last_limits: Option<Arc<RateLimits>>,
    observation_slots: usize,
}

impl SourceDecoder {
    fn new(file_thread: &str) -> Self {
        let mut interner = Interner::default();
        let file_thread = interner.intern(file_thread);
        Self {
            id: None,
            file_thread,
            interner,
            counts: Vec::new(),
            records: Vec::new(),
            metas: SourceMetas::default(),
            skipped: 0,
            last_limits: None,
            observation_slots: 0,
        }
    }

    fn finish(self) -> ParsedSource {
        let Self {
            id,
            file_thread,
            interner,
            mut counts,
            mut records,
            metas,
            skipped,
            observation_slots,
            ..
        } = self;
        // Decoding grows the buffers by doubling; release the unused tails before the
        // records wait for the other rollouts.
        counts.shrink_to_fit();
        records.shrink_to_fit();
        let root = is_root_rollout(&metas, &records, file_thread);
        let counter_scope = classify_counters(&mut records, &counts, root);
        let strings = interner.finish();
        let root_turns = root_turn_digests(&records, &strings, metas.root == Some(true));
        ParsedSource {
            id,
            file_thread,
            strings,
            counts,
            records,
            metas,
            trailing_skipped: skipped,
            counter_scope,
            observation_slots,
            root_turns,
        }
    }

    fn skip(&mut self) -> RecordDisposition {
        self.skipped = self.skipped.saturating_add(1);
        RecordDisposition::Skipped
    }

    #[cfg(test)]
    fn decode(&mut self, raw: &RawRecord<'_>) -> RecordDisposition {
        self.decode_with_admission(raw, None)
    }

    fn decode_with_admission(
        &mut self,
        raw: &RawRecord<'_>,
        admission: Option<&MemoryAdmission>,
    ) -> RecordDisposition {
        if admission.is_some_and(MemoryAdmission::stopped) {
            return RecordDisposition::Stop;
        }
        if !may_be_relevant(raw.bytes) {
            // Validate without building a document: most rollout lines are content the
            // adapter skips, and allocating a JSON tree for each one only fragments the heap.
            return if validate_record(raw.bytes).is_ok() {
                self.skip()
            } else {
                RecordDisposition::Malformed
            };
        }
        let Ok(line) = Line::read(raw.bytes) else {
            return RecordDisposition::Malformed;
        };
        let Some(kind) = self.record_kind(line.record_type, line.payload, raw.evidence) else {
            return self.skip();
        };
        let request_bearing = match &kind {
            RecordKind::UsageRecord(_) => true,
            RecordKind::Compacted(latest) => latest.is_some(),
            RecordKind::TokenCount { usage, .. } => usage.0 != 0,
            RecordKind::SessionMeta { .. }
            | RecordKind::TurnContext { .. }
            | RecordKind::ThreadSettingsApplied { .. } => false,
        };
        if request_bearing && admission.is_some_and(|budget| !budget.reserve_row(Agent::Codex)) {
            return RecordDisposition::Stop;
        }
        // The `src-` ID lives on the manifest entry; evidence carries a local index.
        if can_emit_observation(&kind) {
            self.observation_slots = self.observation_slots.saturating_add(1);
        }
        self.records.push(ParsedRecord {
            offset: raw.evidence.offset,
            length: u64::from(raw.evidence.length),
            skipped_before: std::mem::take(&mut self.skipped),
            ordinal: line.ordinal,
            timestamp: line
                .timestamp
                .as_deref()
                .and_then(|timestamp| parse_timestamp(timestamp).ok())
                .map(CompactTimestamp::from),
            kind,
        });
        RecordDisposition::Decoded
    }

    /// The accounting fields of a relevant record, or `None` for a record the adapter skips.
    fn record_kind(
        &mut self,
        record_type: RecordType,
        payload: Payload<'_>,
        evidence: &EvidenceRef,
    ) -> Option<RecordKind> {
        match record_type {
            RecordType::SessionMeta => Some(self.session_meta(payload, evidence)),
            RecordType::TurnContext => Some(RecordKind::TurnContext {
                turn_id: self.interner.intern_some(payload.turn_id.as_deref()),
                model: payload.model.as_deref().map(Name::new),
                effort: payload.effort.as_deref().map(Name::new),
            }),
            RecordType::TokenUsageRecord => Some(RecordKind::UsageRecord(
                self.usage_record(&payload.usage_record, payload.turn_id.as_deref()),
            )),
            RecordType::Compacted => Some(RecordKind::Compacted(
                payload
                    .latest_token_usage_record
                    .as_ref()
                    .map(|latest| self.usage_record(latest, latest.turn_id.as_deref())),
            )),
            RecordType::EventMsg => match payload.event_type {
                EventType::ThreadSettingsApplied => Some(RecordKind::ThreadSettingsApplied {
                    thread_id: self.interner.intern_some(payload.usage_record.thread_id.as_deref()),
                }),
                EventType::TokenCount => {
                    let mut usage = UsageMask::default();
                    pack_usage(&mut self.counts, &mut usage, 0, payload.total_token_usage);
                    pack_usage(&mut self.counts, &mut usage, 1, payload.last_token_usage);
                    Some(RecordKind::TokenCount {
                        usage,
                        limits: rate_limits(payload.rate_limits, &mut self.last_limits),
                        role: CounterRole::Counted,
                    })
                }
                EventType::Other => None,
            },
            RecordType::Other => None,
        }
    }

    fn usage_record(&mut self, fields: &UsageFields<'_>, turn_id: Option<&str>) -> UsageRecord {
        let mut usage = UsageMask::default();
        pack_usage(&mut self.counts, &mut usage, 0, fields.usage);
        UsageRecord {
            thread_id: self.interner.intern_some(fields.thread_id.as_deref()),
            response_id: self.interner.intern_some(fields.response_id.as_deref()),
            turn: self.interner.intern_some(turn_id.or(fields.root_turn_id.as_deref())),
            usage,
        }
    }

    fn session_meta(&mut self, payload: Payload<'_>, evidence: &EvidenceRef) -> RecordKind {
        let id = self.interner.intern_some(payload.id.as_deref());
        let metas = &mut self.metas;
        if metas.cli_version.is_none() {
            metas.cli_version = payload.cli_version.map(Cow::into_owned);
        }
        let own = id == Some(self.file_thread);
        metas.lineage |= !own
            || payload.parent_thread_id.is_some()
            || payload.forked_from_id.is_some()
            || !matches!(payload.subagent_history_start_ordinal, HistoryBoundary::Missing);
        metas.spawn_parent |= payload.spawn_parent;
        if metas.root.is_none() {
            metas.root =
                Some(own && payload.parent_thread_id.is_none() && payload.forked_from_id.is_none());
        }
        if !own {
            metas.foreign |= id.is_some();
        } else if metas.own.is_none() {
            let meta = SessionMeta {
                source: payload.source.map(Cow::into_owned),
                thread_source: payload.thread_source.map(Cow::into_owned),
                project: payload
                    .cwd
                    .as_deref()
                    .and_then(|cwd| Path::new(cwd).file_name())
                    .map(|name| name.to_string_lossy().into_owned()),
                parent_thread_id: payload.parent_thread_id.map(Cow::into_owned),
                forked_from_id: payload.forked_from_id.map(Cow::into_owned),
                subagent_history_start_ordinal: payload.subagent_history_start_ordinal,
            };
            metas.own = Some(Box::new((meta, *evidence)));
        }
        RecordKind::SessionMeta { id }
    }
}

/// A token count's `rate_limits` object, sharing the previous snapshot when identical.
fn rate_limits(
    rate_limits: Option<DecodedLimits>,
    last: &mut Option<Arc<RateLimits>>,
) -> Option<Arc<RateLimits>> {
    let decoded = rate_limits?;
    if let Some(previous) = last.as_ref().filter(|previous| previous.native == decoded.native) {
        return Some(Arc::clone(previous));
    }
    let limits = Arc::new(RateLimits {
        limit_name: decoded.limit_name,
        windows: decoded.windows,
        native: decoded.native,
    });
    *last = Some(Arc::clone(&limits));
    Some(limits)
}

/// Expands Codex homes to the rollout directories the adapter actually reads.
///
/// A root without the standard home layout is kept as-is so an explicit directory of
/// rollout files remains a valid source.
pub fn rollout_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut expanded = Vec::new();
    for root in roots {
        let candidates = [root.join("sessions"), root.join("archived_sessions")];
        let mut found_layout = false;
        for candidate in candidates {
            if candidate.is_dir() {
                expanded.push(candidate);
                found_layout = true;
            }
        }
        if !found_layout {
            expanded.push(root.clone());
        }
    }
    expanded
}

fn thread_ids_from_locators<'a>(
    locators: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeMap<String, AnalyticalId>, AdapterError> {
    let mut ids = BTreeMap::new();
    for locator in locators {
        let native = rollout_name(locator).thread_id;
        if let std::collections::btree_map::Entry::Vacant(slot) = ids.entry(native) {
            let identity = thread_identity(slot.key())?;
            slot.insert(identity.id);
        }
    }
    Ok(ids)
}

/// Which of a rollout's `token_count` events the cumulative-counter rules account for.
///
/// From `rust-v0.153.0` Codex writes a `token_usage_record` for each response that
/// reports usage, beside the `token_count` it still writes for that response: the record
/// first, then the counter. A root rollout ([`is_root_rollout`]) can also hold
/// counter-only usage beside its usage records: a session that a release before 0.153
/// started and a later one resumed, or one that an earlier release appended turns to after
/// a later one wrote it. [`classify_counters`] decides the [`CounterRole`] of each of its
/// counters once, when it decodes. In any other rollout with usage records, the usage
/// records account for its usage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterScope {
    /// Every one: the rollout has no `token_usage_record`.
    All,
    /// None: the rollout has usage records, and is not a root rollout or reports through
    /// its counters no response that its usage records do not.
    Direct,
    /// Those whose role is [`CounterRole::Counted`]: a root rollout with usage records,
    /// at least one of whose counters reports usage that no usage record reports.
    Mixed,
}

/// How the accounting treats one `token_count` of a root rollout with usage records.
///
/// A counter is a *usage event* when its total differs from the previous counter's and
/// its `last_token_usage` reports input or output; a usage record is always one. Read in
/// order, a usage-event counter is the twin of the usage record just before it (the order
/// Codex writes) or, failing that, of the one just after it (the reverse order, accepted
/// too), when that record has exactly the counter's usage, no earlier counter took it, and
/// no `turn_context` lies between them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CounterRole {
    /// The cumulative-counter rules account for it: every counter of a rollout without
    /// usage records; in a root rollout with them, a usage-event counter that is not a
    /// twin, and a counter that reports no new usage after one (or before the first usage
    /// event, when that is one).
    #[default]
    Counted,
    /// The twin of an adjacent usage record, which accounts for its response.
    Twin,
    /// A counter that reports no new usage after a usage record or a twin, or before the
    /// first usage event when that is one.
    Covered,
}

/// The previous usage event while [`classify_counters`] reads a rollout.
#[derive(Clone, Copy)]
enum UsageEvent {
    /// A usage record with its usage, and whether a counter already took it as its twin.
    Record { usage: Option<CodexUsage>, taken: bool },
    /// A usage-event counter, its last usage, and whether it is a twin.
    Counter { index: usize, last: CodexUsage, twin: bool },
}

/// Whether a `token_count` with this `total` and `last` usage is a usage event, given the
/// previous counter's total.
fn is_usage_event(
    total: Option<&CodexUsage>,
    last: Option<&CodexUsage>,
    previous_total: Option<&CodexUsage>,
) -> bool {
    total.is_some()
        && total != previous_total
        && last.is_some_and(|last| last.input.unwrap_or(0) > 0 || last.output.unwrap_or(0) > 0)
}

/// Whether a rollout cannot hold another thread's history, so every counter and usage
/// record in it is its own thread's: every `session_meta` names its own thread, with
/// neither a parent nor a fork origin and no `subagent_history_start_ordinal`, and no
/// `thread_settings_applied` or usage record names another thread.
fn is_root_rollout(metas: &SourceMetas, records: &[ParsedRecord], file_thread: Sym) -> bool {
    !metas.lineage
        && records.iter().all(|record| match record.kind {
            RecordKind::ThreadSettingsApplied { thread_id } => {
                thread_id.is_none_or(|thread| thread == file_thread)
            }
            RecordKind::UsageRecord(usage) => {
                usage.thread_id.is_none_or(|thread| thread == file_thread)
            }
            RecordKind::SessionMeta { .. }
            | RecordKind::TurnContext { .. }
            | RecordKind::Compacted(_)
            | RecordKind::TokenCount { .. } => true,
        })
}

/// Sets the role of the `token_count` at `index`.
fn set_role(records: &mut [ParsedRecord], index: usize, role: CounterRole) {
    if let Some(ParsedRecord { kind: RecordKind::TokenCount { role: slot, .. }, .. }) =
        records.get_mut(index)
    {
        *slot = role;
    }
}

/// Decides which counters the counter rules account for, and, in a root rollout with
/// usage records, each counter's [`CounterRole`].
fn classify_counters(records: &mut [ParsedRecord], counts: &[u8], root: bool) -> CounterScope {
    if !records.iter().any(|record| matches!(record.kind, RecordKind::UsageRecord(_))) {
        return CounterScope::All;
    }
    if !root {
        return CounterScope::Direct;
    }
    // Twins, pairing each counter with an untaken record next to it within its turn, the
    // one before it first. A counter that reports no new usage is provisionally `Covered`.
    let mut reader = CountReader::new(counts);
    let mut previous_total = None;
    let mut previous = None;
    for index in 0..records.len() {
        let [first, second] = reader.record(&records[index].kind);
        match records[index].kind {
            RecordKind::UsageRecord(_) => {
                let taken = match previous {
                    Some(UsageEvent::Counter { index: counter, last, twin: false })
                        if first == Some(last) =>
                    {
                        set_role(records, counter, CounterRole::Twin);
                        true
                    }
                    Some(UsageEvent::Record { .. } | UsageEvent::Counter { .. }) | None => false,
                };
                previous = Some(UsageEvent::Record { usage: first, taken });
            }
            RecordKind::TokenCount { .. } => {
                let event =
                    is_usage_event(first.as_ref(), second.as_ref(), previous_total.as_ref());
                if first.is_some() {
                    previous_total = first;
                }
                let (true, Some(last)) = (event, second) else {
                    set_role(records, index, CounterRole::Covered);
                    continue;
                };
                let twin = matches!(
                    previous,
                    Some(UsageEvent::Record { usage: Some(usage), taken: false }) if usage == last
                );
                if twin {
                    set_role(records, index, CounterRole::Twin);
                }
                previous = Some(UsageEvent::Counter { index, last, twin });
            }
            // Codex writes a turn's `turn_context` before any of its responses, never
            // between a response's record and its counter, so no pair spans one. A legacy
            // response before a resumed turn then never pairs with that turn's record.
            RecordKind::TurnContext { .. } => previous = None,
            RecordKind::SessionMeta { .. }
            | RecordKind::Compacted(_)
            | RecordKind::ThreadSettingsApplied { .. } => {}
        }
    }
    // A counter that reports no new usage follows the nearest usage event before it, or
    // the first one when none precedes it.
    let event_role = |kind: &RecordKind| match kind {
        RecordKind::TokenCount { role: CounterRole::Counted, .. } => Some(CounterRole::Counted),
        RecordKind::UsageRecord(_) | RecordKind::TokenCount { role: CounterRole::Twin, .. } => {
            Some(CounterRole::Covered)
        }
        RecordKind::TokenCount { role: CounterRole::Covered, .. }
        | RecordKind::SessionMeta { .. }
        | RecordKind::TurnContext { .. }
        | RecordKind::Compacted(_)
        | RecordKind::ThreadSettingsApplied { .. } => None,
    };
    let mut current =
        records.iter().find_map(|record| event_role(&record.kind)).unwrap_or(CounterRole::Covered);
    let mut counted = false;
    for record in records.iter_mut() {
        if let Some(role) = event_role(&record.kind) {
            counted |= role == CounterRole::Counted;
            current = role;
        } else if let RecordKind::TokenCount { role, .. } = &mut record.kind {
            *role = current;
        }
    }
    if counted { CounterScope::Mixed } else { CounterScope::Direct }
}

fn decode_rollout(
    source: &DiscoveredSource,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    admission: &MemoryAdmission,
) -> Result<(ManifestEntry, DecodedRollout), AdapterError> {
    let (entry, parsed) = decode_source(source, admission)?;
    // Only a rollout without usage records may infer where a legacy copied prefix ends
    // from other rollouts' turn IDs, so only it waits for every rollout to decode.
    if parsed.counter_scope == CounterScope::All {
        Ok((entry, DecodedRollout::Pending(parsed)))
    } else {
        Ok((
            entry,
            DecodedRollout::Observed(observe_to_observed(
                parsed,
                thread_ids,
                &ParentEvidence::default(),
            )?),
        ))
    }
}

/// Where a rollout's own records start, from its own `subagent_history_start_ordinal`.
///
/// The boundary places only lines that name no thread: a usage record names the thread
/// that made its request, and that decides its owner wherever it sits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeBoundary {
    /// None is declared; ownership follows session headers and settings events.
    Absent,
    /// Lines before this ordinal are inherited history, unless an earlier line names their
    /// owner. A natively created child's inherited prefix never names the child, while
    /// Codex's legacy-to-paginated migration puts the boundary after every line it
    /// migrated, the child's own included.
    At(u64),
    /// The declared value is not an ordinal, so it cannot place any record.
    Invalid,
}

impl NativeBoundary {
    fn of(meta: Option<&SessionMeta>) -> Self {
        match meta.map(|meta| meta.subagent_history_start_ordinal) {
            None | Some(HistoryBoundary::Missing) => Self::Absent,
            Some(HistoryBoundary::Ordinal(ordinal)) => Self::At(ordinal),
            Some(HistoryBoundary::Invalid) => Self::Invalid,
        }
    }

    fn is_declared(self) -> bool {
        self != Self::Absent
    }

    /// Whether a record at `ordinal` is inherited history before the boundary.
    fn precedes(self, ordinal: Option<u64>) -> bool {
        match self {
            Self::At(boundary) => ordinal.is_some_and(|ordinal| ordinal < boundary),
            Self::Absent | Self::Invalid => false,
        }
    }

    /// Whether a declared boundary leaves a record at `ordinal` on neither side of it.
    fn cannot_place(self, ordinal: Option<u64>) -> bool {
        match self {
            Self::Absent => false,
            Self::At(_) => ordinal.is_none(),
            Self::Invalid => true,
        }
    }
}

/// What the turns before a declared boundary show about the unnamed lines that no earlier
/// line assigns, in a counter-only rollout that holds no other thread's `session_meta`.
///
/// Codex's migration keeps `turn_context` lines and a child's own turn IDs are its own, so
/// a turn the parent root recorded is copied history, and the first turn it never recorded
/// starts the child's own lines.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrefixTurn {
    /// No `turn_context` yet, so nothing places the lines.
    None,
    /// A turn the parent root recorded: the lines are the parent's copy.
    Parent,
    /// A turn the parent's turns cannot place: it has no ID (before `rust-v0.100.0`), or
    /// the parent is not a discovered root.
    Unplaceable,
}

/// Where turn inference started a rollout's own lines before its boundary: the
/// observations and diagnostics built until then, and the copied total it continued.
struct InferredStart {
    observations: usize,
    diagnostics: usize,
    baseline: Option<TokenMeasures>,
}

/// Whether a rollout decides the unassigned lines before its declared boundary by their
/// turns: a counter-only rollout that holds no other thread's `session_meta`. A native
/// subagent's inherited prefix keeps its parent's header, and Codex's migration drops
/// every one but the rollout's own.
fn infers_prefix_turns(source: &ParsedSource) -> bool {
    let meta = source.metas.own.as_deref().map(|(meta, _)| meta);
    source.counter_scope == CounterScope::All
        && !source.metas.foreign
        && matches!(NativeBoundary::of(meta), NativeBoundary::At(_))
}

/// Whether two usage objects report the same counts, reading a missing count as 0.
fn same_counts(left: &CodexUsage, right: &CodexUsage) -> bool {
    left.counts()
        .into_iter()
        .zip(right.counts())
        .all(|(left, right)| left.unwrap_or(0) == right.unwrap_or(0))
}

/// Whether `total` reports more of any category than `accounted`.
fn exceeds(total: &TokenMeasures, accounted: &TokenMeasures) -> bool {
    total
        .categories()
        .into_iter()
        .zip(accounted.categories())
        .any(|((_, total), (_, accounted))| total.unwrap_or(0) > accounted.unwrap_or(0))
}

/// Explains a [`DiagnosticCode::CodexHistoryBoundaryUnverified`] without naming a path or
/// any value from the rollout.
const UNVERIFIED_BOUNDARY_DETAIL: &str = concat!(
    "Codex fork-boundary evidence could not prove which usage is this thread's own, so that ",
    "usage is excluded as a coverage gap; inspect the rollout's ",
    "subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's ",
    "and its token_count totals",
);

/// Explains an unseeded child counter that starts a new epoch above its inherited total.
const UNSEEDED_COUNTER_DETAIL: &str =
    "Codex cumulative usage did not continue the inherited total and opened a new counter epoch";

/// Usage records whose ownership a declared fork boundary could not establish.
#[derive(Default)]
struct UnverifiedUsage {
    /// The first few records, as diagnostic and gap samples.
    evidence: Vec<EvidenceRef>,
    occurrences: u64,
}

impl UnverifiedUsage {
    fn add(&mut self, evidence: EvidenceRef) {
        if self.evidence.len() < SAMPLE_EVIDENCE_LIMIT {
            self.evidence.push(evidence);
        }
        self.occurrences = self.occurrences.saturating_add(1);
    }
}

/// How a rollout's first own cumulative total relates to the baseline it may continue.
enum FirstStep {
    /// The baseline accounts for the total, or the step reports no usage it could check:
    /// count from it.
    Continues(RunningTotal),
    /// The total repeats the baseline, so the step adds nothing and the next step that
    /// reports usage is still the one to check.
    Repeated,
    /// The total is the step's own usage alone, so the inherited baseline does not apply.
    Unseeded,
    /// The total fell below the baseline and its own usage is zero: it reports no usage,
    /// so the next step is checked, from this total.
    Lowered,
    /// Neither the baseline nor a zero start accounts for the total.
    Unverified,
}

/// Checks a first own total against `last`, the usage of the request it reports.
///
/// A seeded child continues its parent's running total, so its first delta from the
/// inherited baseline equals `last`; an unseeded one starts from zero, so its total does.
/// Without an inherited baseline, only the zero start can be proven.
fn first_step(
    inherited: Option<TokenMeasures>,
    total: &TokenMeasures,
    last: Option<&TokenMeasures>,
) -> Result<FirstStep, AdapterError> {
    let tracker = inherited.map_or_else(RunningTotal::new, RunningTotal::inheriting);
    let Some(last) = last else { return Ok(FirstStep::Continues(tracker)) };
    let step = tracker.clone().observe(total, None)?;
    let proven = match step.event {
        CounterEvent::Repeated => return Ok(FirstStep::Repeated),
        CounterEvent::Reset
            if !same_usage(total, last) && same_usage(last, &TokenMeasures::default()) =>
        {
            return Ok(FirstStep::Lowered);
        }
        CounterEvent::Reset => same_usage(total, last),
        CounterEvent::Advanced | CounterEvent::Gap { .. } => same_usage(&step.delta, last),
    };
    Ok(if proven {
        FirstStep::Continues(tracker)
    } else if same_usage(total, last) {
        FirstStep::Unseeded
    } else {
        FirstStep::Unverified
    })
}

/// Whether `total` changes the running total the rollout's next own step continues: the
/// counter's, or else the inherited total. A total that only repeats it carries no usage.
fn changes_running_total(
    counter: Option<&RunningTotal>,
    inherited: Option<TokenMeasures>,
    total: &TokenMeasures,
) -> Result<bool, AdapterError> {
    let mut tracker = counter
        .cloned()
        .unwrap_or_else(|| inherited.map_or_else(RunningTotal::new, RunningTotal::inheriting));
    Ok(tracker.observe(total, None)?.event != CounterEvent::Repeated)
}

/// Category-wise equality, reading a missing count as zero.
fn same_usage(left: &TokenMeasures, right: &TokenMeasures) -> bool {
    left.categories()
        .into_iter()
        .zip(right.categories())
        .all(|((_, left), (_, right))| left.unwrap_or(0) == right.unwrap_or(0))
}

fn observe_parsed_source(
    source: &ParsedSource,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    parents: &ParentEvidence,
) -> Result<SourceObserve, AdapterError> {
    let mut observations = Vec::with_capacity(source.observation_slots);
    let mut limit_observations = Vec::new();
    let mut diagnostics = Vec::new();
    let mut gaps = Vec::new();
    let mut copied_regions = 0_u64;
    let Some(_source_id) = source.id.as_ref() else {
        return Ok(SourceObserve {
            observations,
            limit_observations,
            diagnostics,
            gaps,
            copied_regions,
        });
    };
    let strings = &source.strings;
    let file_thread = source.file_thread;
    let file_thread_text = strings.resolve(file_thread);
    let scope = source.counter_scope;
    let own_meta = source.metas.own.as_deref().map(|(meta, _)| meta);
    let parent_thread = own_meta.and_then(SessionMeta::parent);
    let native_boundary = NativeBoundary::of(own_meta);
    let has_foreign_meta = source.metas.foreign;
    // A natively created child writes its own settings event at the boundary, while Codex's
    // migration puts the boundary one past the last line it migrated, so no line of a
    // migrated rollout that was never resumed reaches it.
    let boundary_unreached = match native_boundary {
        NativeBoundary::At(boundary) => !source
            .records
            .iter()
            .any(|record| record.ordinal.is_some_and(|ordinal| ordinal >= boundary)),
        NativeBoundary::Absent | NativeBoundary::Invalid => false,
    };
    let infers_turns = infers_prefix_turns(source);
    let complete_parent = parent_thread.filter(|parent| parents.complete.contains(*parent));
    let parent_turns = complete_parent.and_then(|parent| parents.turns.get(parent));
    let parent_totals = complete_parent.and_then(|parent| parents.totals.get(parent));
    // The responses this rollout's own usage records report: a `compacted` line naming this
    // rollout's thread copies one of them, or else keeps the only record of its response,
    // as when a bounded migration dropped every line before the compaction.
    let own_responses: HashSet<Sym> = if source.records.iter().any(|record| {
        matches!(record.kind, RecordKind::Compacted(Some(usage)) if usage.thread_id == Some(file_thread))
    }) {
        source
            .records
            .iter()
            .filter_map(|record| match record.kind {
                RecordKind::UsageRecord(usage) if usage.thread_id == Some(file_thread) => {
                    usage.response_id
                }
                RecordKind::UsageRecord(_)
                | RecordKind::SessionMeta { .. }
                | RecordKind::TurnContext { .. }
                | RecordKind::Compacted(_)
                | RecordKind::TokenCount { .. }
                | RecordKind::ThreadSettingsApplied { .. } => None,
            })
            .collect()
    } else {
        HashSet::new()
    };
    if parent_thread.is_some()
        && has_foreign_meta
        && (scope == CounterScope::All || native_boundary.is_declared())
    {
        copied_regions = copied_regions.saturating_add(1);
        if scope == CounterScope::All && !native_boundary.is_declared() {
            let copied = legacy_copied_evidence(source, &parents.turns);
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::CodexCopiedHistoryInferred,
                    thread_ids.get(file_thread_text).cloned(),
                    copied.evidence,
                    "Codex copied-history boundary was inferred from legacy rollout records",
                )
                .with_occurrences(copied.occurrences),
            );
        }
    }
    let mut active_thread = file_thread;
    // Whether the latest line that named a thread named this rollout's: a settings event,
    // a usage record or a compacted line's record does, and a session header names its own
    // thread before a native prefix, so it does not. Before a declared boundary, only such
    // a line, or the first turn the parent root never recorded, gives the rollout the
    // unnamed lines after it.
    let mut named_own = false;
    // What the turns so far show about unassigned lines before the boundary.
    let mut prefix_turn = PrefixTurn::None;
    // A child's own lines never precede a turn or a total its parent recorded, or a line
    // naming another thread, so after a turn-inferred start any of these before the
    // boundary proves the inference wrong, and everything from the start to the boundary
    // is then undecided.
    let mut inferred: Option<InferredStart> = None;
    let mut contradicted = false;
    // The latest running total a counter the counter rules read reported.
    let mut last_total: Option<TokenMeasures> = None;
    // A `compacted` record counted as this rollout's original, whose response's counter, if
    // one follows in a rollout without usage records, must not count it again.
    let mut compacted_twin: Option<CodexUsage> = None;
    // The usage this rollout's own originals report so far, and whether its first running
    // total after a compaction that alone keeps its own record is still to be checked
    // against it.
    let mut accounted = TokenMeasures::default();
    let mut check_dropped = false;
    // Whether the declared boundary leaves usage-bearing lines to another thread or to none.
    let mut copied_prefix = false;
    let mut turns = Turns::new();
    let mut current_turn: Option<Sym> = None;
    let mut last_response_by_thread: HashMap<&str, Sym> = HashMap::new();
    let mut inherited_total = None;
    let mut counter = None;
    let mut unverified = UnverifiedUsage::default();
    let mut previous_limits = BTreeMap::new();
    let mut counts = CountReader::new(&source.counts);
    for record in &source.records {
        let [first_usage, second_usage] = counts.record(&record.kind);
        let view = RecordView { evidence: record.evidence(0), timestamp: record.timestamp };
        // Whether the counter rules count this record: any `token_count` of a rollout without
        // usage records, and a counted one of a root rollout with them.
        let cumulative = matches!(
            record.kind,
            RecordKind::TokenCount { role, .. } if scope == CounterScope::All
                || (scope == CounterScope::Mixed && role == CounterRole::Counted)
        );
        if let NativeBoundary::At(boundary) = native_boundary {
            if record.ordinal.is_some_and(|ordinal| ordinal >= boundary) {
                active_thread = file_thread;
            }
        }
        // A paginated prefix need not contain a foreign session header: an explicit
        // ordinal boundary alone assigns the unnamed lines before it to the parent, unless
        // an earlier line names this rollout's thread, as the child's own lines do in a
        // migrated rollout. It establishes ownership of the inherited prefix, not a counter
        // baseline.
        let before_boundary = native_boundary.precedes(record.ordinal);
        if let Some(start) = inferred.take() {
            // The window stays open until a line at or past the boundary, or a line naming
            // this rollout's own thread, after which the lines are the child's by name. A
            // line without an ordinal does not say which side of the boundary it is on.
            let past_boundary = match native_boundary {
                NativeBoundary::At(boundary) => {
                    record.ordinal.is_some_and(|ordinal| ordinal >= boundary)
                }
                NativeBoundary::Absent | NativeBoundary::Invalid => true,
            };
            // A line naming another thread is copied history, which never follows the
            // child's own lines.
            let names_other = match &record.kind {
                RecordKind::SessionMeta { id } => id.is_some_and(|thread| thread != file_thread),
                RecordKind::ThreadSettingsApplied { thread_id } => {
                    thread_id.is_some_and(|thread| thread != file_thread)
                }
                RecordKind::UsageRecord(usage) | RecordKind::Compacted(Some(usage)) => {
                    usage.thread_id.is_some_and(|thread| thread != file_thread)
                }
                RecordKind::TurnContext { .. }
                | RecordKind::Compacted(None)
                | RecordKind::TokenCount { .. } => false,
            };
            let reported_total = match (&record.kind, first_usage.as_ref()) {
                (RecordKind::TokenCount { .. }, Some(total)) => {
                    let repeats_baseline = match &start.baseline {
                        Some(baseline) => same_usage(&codex_usage(total)?, baseline),
                        None => false,
                    };
                    !repeats_baseline
                        && parent_totals.is_some_and(|totals| totals.contains(&total_digest(total)))
                }
                (_, _) => false,
            };
            let recorded_turn = match &record.kind {
                RecordKind::TurnContext { turn_id: Some(turn), .. } => {
                    parent_turns.is_some_and(|recorded| {
                        recorded.contains(&turn_digest(strings.resolve(*turn)))
                    })
                }
                RecordKind::TurnContext { turn_id: None, .. }
                | RecordKind::SessionMeta { .. }
                | RecordKind::UsageRecord(_)
                | RecordKind::Compacted(_)
                | RecordKind::TokenCount { .. }
                | RecordKind::ThreadSettingsApplied { .. } => false,
            };
            if !past_boundary && (reported_total || recorded_turn || names_other) {
                // Every own step counted since the start becomes undecided usage.
                for observation in observations.split_off(start.observations) {
                    if observation.role == ObservationRole::Original {
                        unverified.add(observation.evidence);
                    } else {
                        observations.push(observation);
                    }
                }
                diagnostics.truncate(start.diagnostics);
                contradicted = true;
                named_own = false;
                prefix_turn = PrefixTurn::Unplaceable;
                counter = None;
                inherited_total = last_total.or(inherited_total);
            } else if !past_boundary {
                inferred = Some(start);
            }
        }
        let inherited = before_boundary && active_thread == file_thread && !named_own;
        let owner = if inherited { parent_thread } else { Some(strings.resolve(active_thread)) };
        // An inherited line in a rollout without another thread's header that no turn of
        // the parent places may be the child's migrated usage or a copied prefix. It is
        // undecided where the region shows migrated content: a turn the parent's turns
        // cannot place, or a boundary no line reaches. Where a line reaches the boundary
        // and no turn precedes, it stays inherited, as in a native prefix.
        // A counter whose cumulative total the parent root's own counters report is that
        // counter's copy, whatever turn it sits in.
        let copied_total = matches!(record.kind, RecordKind::TokenCount { .. })
            && parent_totals
                .zip(first_usage.as_ref())
                .is_some_and(|(totals, total)| totals.contains(&total_digest(total)));
        let undecided = inherited
            && !has_foreign_meta
            && prefix_turn != PrefixTurn::Parent
            && !copied_total
            && (boundary_unreached || prefix_turn == PrefixTurn::Unplaceable);
        // Usage that would count as this rollout's own, but that a declared boundary
        // cannot place, is copied history of no proven owner: never the child's usage.
        // A record that names a thread needs no placing.
        // A cumulative total that only repeats the running total carries no usage; Codex
        // re-sends the current totals with every rate-limit refresh.
        let unplaced = (native_boundary.cannot_place(record.ordinal) || undecided)
            && match &record.kind {
                RecordKind::UsageRecord(usage) => {
                    usage.thread_id.is_none() && active_thread == file_thread
                }
                RecordKind::TokenCount { .. } => match &first_usage {
                    Some(total) if cumulative && active_thread == file_thread => {
                        changes_running_total(
                            counter.as_ref(),
                            inherited_total,
                            &codex_usage(total)?,
                        )?
                    }
                    Some(_) | None => false,
                },
                RecordKind::SessionMeta { .. }
                | RecordKind::TurnContext { .. }
                | RecordKind::Compacted(_)
                | RecordKind::ThreadSettingsApplied { .. } => false,
            };
        if unplaced {
            unverified.add(view.evidence);
        }
        let usage_owner = if unplaced { None } else { owner };
        // A usage-bearing line the boundary leaves to another thread, or to none, is part of
        // a copied prefix; a line this rollout owns is not, wherever it sits.
        copied_prefix |= can_emit_observation(&record.kind)
            && match native_boundary {
                NativeBoundary::Absent => false,
                NativeBoundary::At(_) => before_boundary,
                NativeBoundary::Invalid => true,
            }
            && match &record.kind {
                RecordKind::UsageRecord(usage) | RecordKind::Compacted(Some(usage)) => usage
                    .thread_id
                    .map_or(usage_owner != Some(file_thread_text), |thread| thread != file_thread),
                RecordKind::TokenCount { .. } => usage_owner != Some(file_thread_text),
                // A compacted line without a record carries no usage, and the other kinds
                // emit no observation.
                RecordKind::Compacted(None)
                | RecordKind::SessionMeta { .. }
                | RecordKind::TurnContext { .. }
                | RecordKind::ThreadSettingsApplied { .. } => false,
            };
        match &record.kind {
            RecordKind::SessionMeta { id } => {
                if let Some(thread_id) = *id {
                    active_thread = thread_id;
                    named_own = false;
                    inferred = None;
                }
            }
            RecordKind::TurnContext { turn_id, model, effort } => {
                if scope == CounterScope::All
                    && active_thread != file_thread
                    && turn_id.is_some_and(|turn| {
                        is_unknown_turn(&parents.turns, strings, active_thread, turn)
                    })
                {
                    active_thread = file_thread;
                }
                if infers_turns
                    && !contradicted
                    && before_boundary
                    && active_thread == file_thread
                    && !named_own
                {
                    match (*turn_id, parent_turns) {
                        (Some(turn), Some(recorded)) => {
                            if recorded.contains(&turn_digest(strings.resolve(turn))) {
                                prefix_turn = PrefixTurn::Parent;
                            } else {
                                // The first turn the parent never recorded starts this
                                // rollout's own lines; its first counter step is still
                                // checked against the copied total.
                                named_own = true;
                                // A later unrecorded turn, as after a thread-less settings
                                // event, never moves an open start.
                                if inferred.is_none() {
                                    inferred = Some(InferredStart {
                                        observations: observations.len(),
                                        diagnostics: diagnostics.len(),
                                        baseline: inherited_total,
                                    });
                                }
                            }
                        }
                        (None, _) | (Some(_), None) => prefix_turn = PrefixTurn::Unplaceable,
                    }
                }
                if let Some(turn_id) = *turn_id {
                    turns.insert(turn_id, TurnContext { model: *model, effort: *effort });
                }
                current_turn = *turn_id;
            }
            RecordKind::ThreadSettingsApplied { thread_id } => {
                active_thread = thread_id.unwrap_or(file_thread);
                named_own = *thread_id == Some(file_thread);
                if thread_id.is_some() {
                    inferred = None;
                }
            }
            RecordKind::UsageRecord(usage_record) => {
                // Codex writes the thread that made the request into its record, and a copy
                // keeps its original's, so a record naming this rollout's thread is its own
                // original and one naming another thread is a copy, wherever either sits.
                // An unnamed record takes the owner its position gives it.
                let mut payload = usage_record.payload(strings, first_usage);
                payload.thread_id = payload.thread_id.or(usage_owner);
                let role = if unplaced { ObservationRole::Copy } else { ObservationRole::Original };
                let observation =
                    usage_observation(&view, &payload, role, file_thread_text, thread_ids, &turns)?;
                if let (Some(response_id), Some(owner)) =
                    (usage_record.response_id, payload.thread_id)
                {
                    last_response_by_thread.insert(owner, response_id);
                }
                if observation.role == ObservationRole::Original {
                    if let Some(usage) = &first_usage {
                        accounted = accounted.checked_add(&codex_usage(usage)?)?;
                    }
                }
                observations.push(observation);
                // The counter Codex writes after a record reports the same response, so the
                // lines after a record follow its thread.
                if let Some(thread) = usage_record.thread_id {
                    active_thread = thread;
                    named_own = thread == file_thread;
                    inferred = None;
                }
            }
            RecordKind::Compacted(latest) => {
                if let Some(latest) = latest {
                    let mut payload = latest.payload(strings, first_usage);
                    payload.thread_id = payload.thread_id.or(owner);
                    // A compacted line copies the latest record. When it names this
                    // rollout's thread and no usage record here reports its response, it is
                    // the only record left of that response, as after a bounded migration,
                    // so it is the original; any other original of the response has the
                    // same response key and merges with it.
                    let original = latest.thread_id == Some(file_thread)
                        && latest
                            .response_id
                            .is_some_and(|response| !own_responses.contains(&response));
                    observations.push(usage_observation(
                        &view,
                        &payload,
                        if original { ObservationRole::Original } else { ObservationRole::Copy },
                        file_thread_text,
                        thread_ids,
                        &turns,
                    )?);
                    if original {
                        if let Some(usage) = &first_usage {
                            accounted = accounted.checked_add(&codex_usage(usage)?)?;
                        }
                        // A migration that kept only the suffix from this compaction dropped
                        // every earlier line, so a later running total beyond what this
                        // rollout's own records report shows responses it dropped.
                        check_dropped |= before_boundary && scope == CounterScope::Direct;
                        if scope == CounterScope::All {
                            compacted_twin = first_usage;
                        }
                    }
                    // A Guardian checkpoint writes the counter of the record it keeps right
                    // after it, so the lines after it follow the record's thread too.
                    if let Some(thread) = latest.thread_id {
                        active_thread = thread;
                        named_own = thread == file_thread;
                        inferred = None;
                        if let (Some(response_id), Some(owner)) =
                            (latest.response_id, payload.thread_id)
                        {
                            last_response_by_thread.insert(owner, response_id);
                        }
                    }
                }
            }
            RecordKind::TokenCount { limits, .. } => {
                let (total, last) = (first_usage, second_usage);
                // The counter of a response that a compacted original already reports adds
                // no usage, though it still moves the running total.
                let twin = compacted_twin
                    .is_some_and(|usage| last.is_some_and(|last| same_counts(&last, &usage)));
                if last
                    .is_some_and(|last| last.input.unwrap_or(0) > 0 || last.output.unwrap_or(0) > 0)
                {
                    compacted_twin = None;
                }
                if let Some(total) = total.as_ref().filter(|_| check_dropped) {
                    check_dropped = false;
                    if exceeds(&codex_usage(total)?, &accounted) {
                        unverified.add(view.evidence);
                    }
                }
                if let Some(limits) = limits {
                    append_limits(
                        &view,
                        limits,
                        owner,
                        thread_ids,
                        &mut previous_limits,
                        &mut limit_observations,
                    );
                }
                if !cumulative && owner != Some(file_thread_text) {
                    if let Some(usage) = &last {
                        let response = owner
                            .and_then(|owner| last_response_by_thread.get(owner))
                            .map(|response| strings.resolve(*response));
                        let context = current_turn.and_then(|turn| turns.get(&turn));
                        observations.push(copied_counter_observation(
                            &view, usage, owner, response, thread_ids, context,
                        )?);
                    }
                } else if cumulative {
                    let Some(total) = &total else { continue };
                    let total_usage = codex_usage(total)?;
                    last_total = Some(total_usage);
                    let last = last.as_ref();
                    let silent = last.is_some_and(|last| {
                        last.input.unwrap_or(0) == 0 && last.output.unwrap_or(0) == 0
                    });
                    let estimated = silent && last.is_some_and(|last| last.total.unwrap_or(0) > 0);
                    if unplaced {
                        // The next own step is checked against this total, so usage this
                        // record may carry never reaches that step's delta.
                        counter = None;
                    }
                    let mut copy_owner =
                        (usage_owner != Some(file_thread_text)).then_some(usage_owner);
                    let needs_baseline = inherited_total.is_some()
                        || matches!(native_boundary, NativeBoundary::At(_));
                    if copy_owner.is_none() && counter.is_none() && needs_baseline {
                        let last_usage =
                            last.filter(|_| !estimated).map(codex_usage).transpose()?;
                        match first_step(inherited_total, &total_usage, last_usage.as_ref())? {
                            FirstStep::Continues(tracker) => counter = Some(tracker),
                            FirstStep::Repeated => continue,
                            FirstStep::Unseeded => {
                                diagnostics.push(Diagnostic::new(
                                    DiagnosticCode::CodexCounterEpochReset,
                                    thread_ids.get(file_thread_text).cloned(),
                                    [view.evidence],
                                    UNSEEDED_COUNTER_DETAIL,
                                ));
                                counter = Some(RunningTotal::new());
                            }
                            FirstStep::Lowered => {
                                diagnostics.push(Diagnostic::new(
                                    DiagnosticCode::CodexCounterEpochReset,
                                    thread_ids.get(file_thread_text).cloned(),
                                    [view.evidence],
                                    "Codex cumulative usage decreased and opened a new counter epoch",
                                ));
                                inherited_total = Some(total_usage);
                                continue;
                            }
                            FirstStep::Unverified => {
                                unverified.add(view.evidence);
                                copy_owner = Some(None);
                            }
                        }
                    }
                    if let Some(copy_owner) = copy_owner {
                        inherited_total = Some(total_usage);
                        if let Some(last) = last {
                            observations.push(counter_observation(
                                &view,
                                last,
                                total,
                                CounterObservation {
                                    role: ObservationRole::Copy,
                                    owner: copy_owner,
                                    thread_ids,
                                    context: current_turn.and_then(|turn| turns.get(&turn)),
                                    delta: None,
                                },
                            )?);
                        }
                        continue;
                    }

                    let tracker = counter.get_or_insert_with(|| {
                        inherited_total.map_or_else(RunningTotal::new, RunningTotal::inheriting)
                    });
                    let step = tracker.observe(&total_usage, None)?;
                    if step.event == CounterEvent::Reset {
                        diagnostics.push(Diagnostic::new(
                            DiagnosticCode::CodexCounterEpochReset,
                            thread_ids.get(file_thread_text).cloned(),
                            [view.evidence],
                            "Codex cumulative usage decreased and opened a new counter epoch",
                        ));
                    }
                    let Some(last) = last else { continue };
                    if estimated {
                        let context_fill = total.input.unwrap_or(0) == 0
                            && total.output.unwrap_or(0) == 0
                            && total.total.unwrap_or(0) > 0;
                        diagnostics.push(Diagnostic::new(
                            if context_fill {
                                DiagnosticCode::CodexEstimateContextWindowFill
                            } else {
                                DiagnosticCode::CodexEstimateCompaction
                            },
                            thread_ids.get(file_thread_text).cloned(),
                            [view.evidence],
                            "Codex emitted an estimate with zero input and output tokens",
                        ));
                        continue;
                    }
                    // Codex lowers its running total at compaction rather than restarting
                    // it, so after a decrease the new total is not one request's usage:
                    // the record's own last usage is, which the observation takes when it
                    // has no delta, and a decrease whose last usage has no input or output
                    // reports no new response. Where the counter did restart from zero,
                    // the new total and the last usage agree.
                    let charged = match step.event {
                        CounterEvent::Repeated => false,
                        CounterEvent::Reset => !silent,
                        CounterEvent::Advanced | CounterEvent::Gap { .. } => true,
                    };
                    if charged && !twin {
                        observations.push(counter_observation(
                            &view,
                            last,
                            total,
                            CounterObservation {
                                role: ObservationRole::Original,
                                owner: Some(file_thread_text),
                                thread_ids,
                                context: current_turn.and_then(|turn| turns.get(&turn)),
                                delta: (step.event != CounterEvent::Reset).then_some(step.delta),
                            },
                        )?);
                    }
                } else if let Some(total) = total.as_ref().filter(|_| scope == CounterScope::Mixed)
                {
                    // In a root rollout, usage records account for this counter's response, so
                    // it adds no usage; it still moves the running total a later counted
                    // counter continues, as a counted step would, or that counter's delta
                    // would include it.
                    counter
                        .get_or_insert_with(|| {
                            inherited_total.map_or_else(RunningTotal::new, RunningTotal::inheriting)
                        })
                        .observe(&codex_usage(total)?, None)?;
                }
            }
        }
    }
    if parent_thread.is_some() && !has_foreign_meta && copied_prefix {
        copied_regions = copied_regions.saturating_add(1);
    }
    if unverified.occurrences > 0 {
        let subject = thread_ids.get(file_thread_text).cloned();
        let evidence = unverified.evidence;
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::CodexHistoryBoundaryUnverified,
                subject.clone(),
                evidence.iter().copied(),
                UNVERIFIED_BOUNDARY_DETAIL,
            )
            .with_occurrences(unverified.occurrences),
        );
        gaps.push(CoverageGap {
            reason: UnobservedReason::UnverifiedHistoryBoundary,
            thread: subject,
            evidence,
        });
    }
    Ok(SourceObserve { observations, limit_observations, diagnostics, gaps, copied_regions })
}

fn observe_to_observed(
    source: ParsedSource,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    parents: &ParentEvidence,
) -> Result<ObservedSource, AdapterError> {
    let file_thread = source.strings.resolve(source.file_thread).to_owned();
    let observed = observe_parsed_source(&source, thread_ids, parents)?;
    Ok(ObservedSource {
        id: source.id,
        file_thread,
        metas: source.metas,
        root_turns: source.root_turns,
        observations: observed.observations,
        limit_observations: observed.limit_observations,
        diagnostics: observed.diagnostics,
        gaps: observed.gaps,
        copied_regions: observed.copied_regions,
    })
}

fn rollout_id(rollout: &DecodedRollout) -> Option<&AnalyticalId> {
    match rollout {
        DecodedRollout::Observed(observed) => observed.id.as_ref(),
        DecodedRollout::Pending(source) => source.id.as_ref(),
    }
}

fn source_index(table: &SourceTable, id: Option<&AnalyticalId>) -> u32 {
    id.and_then(|id| table.index_of(id)).unwrap_or(0)
}

fn stamp_refs(
    observations: &mut [RequestObservation],
    limits: &mut [ProviderLimitObservation],
    diagnostics: &mut [Diagnostic],
    gaps: &mut [CoverageGap],
    source: u32,
) {
    for observation in observations {
        observation.evidence = observation.evidence.with_source(source);
    }
    for limit in limits {
        limit.evidence = limit.evidence.with_source(source);
    }
    for diagnostic in diagnostics {
        for evidence in &mut diagnostic.evidence {
            *evidence = evidence.with_source(source);
        }
    }
    for gap in gaps {
        for evidence in &mut gap.evidence {
            *evidence = evidence.with_source(source);
        }
    }
}

fn normalize(
    rollouts: Vec<DecodedRollout>,
    mut manifest: SnapshotManifest,
    mut thread_ids: BTreeMap<String, AnalyticalId>,
    capacity: &ObservationCapacity,
) -> Result<Ingested, AdapterError> {
    let source_table = SourceTable::from_ids(
        rollouts.iter().filter_map(rollout_id).cloned().chain(
            manifest
                .entries
                .iter()
                .filter_map(|entry| entry.source.as_ref().map(|source| source.id.clone())),
        ),
    );
    for entry in &mut manifest.entries {
        let Some(id) = entry.source.as_ref() else { continue };
        let Some(index) = source_table.index_of(&id.id) else { continue };
        if let Some(evidence) = &mut entry.first_malformed {
            *evidence = evidence.with_source(index);
        }
    }
    let mut source_versions: BTreeMap<AnalyticalId, String> = BTreeMap::new();
    let mut native_threads: BTreeSet<String> = BTreeSet::new();
    // Session metadata is copied out, one per thread, so each rollout's records can be
    // freed as soon as its observations are built.
    let mut meta_by_thread: BTreeMap<String, (SessionMeta, EvidenceRef)> = BTreeMap::new();
    for rollout in &rollouts {
        let (id, file_thread, metas) = match rollout {
            DecodedRollout::Observed(observed) => {
                (observed.id.as_ref(), observed.file_thread.as_str(), &observed.metas)
            }
            DecodedRollout::Pending(source) => {
                (source.id.as_ref(), source.strings.resolve(source.file_thread), &source.metas)
            }
        };
        if let (Some(id), Some(version)) = (id, &metas.cli_version) {
            source_versions.entry(id.clone()).or_insert_with(|| version.clone());
        }
        native_threads.insert(file_thread.to_owned());
        if let Some(own) = &metas.own {
            let evidence = own.1.with_source(source_index(&source_table, id));
            meta_by_thread
                .entry(file_thread.to_owned())
                .or_insert_with(|| (own.0.clone(), evidence));
        }
    }
    for native in &native_threads {
        if let std::collections::btree_map::Entry::Vacant(slot) = thread_ids.entry(native.clone()) {
            slot.insert(thread_identity(native)?.id);
        }
    }

    let mut threads = BTreeMap::new();
    for native in native_threads {
        let identity = thread_identity(&native)?;
        thread_ids.insert(native.clone(), identity.id.clone());
        let meta = meta_by_thread.get(&native).map(|(meta, _)| meta);
        let mut native_key = BTreeMap::new();
        native_key.insert("thread_id".to_owned(), native.clone());
        threads.insert(
            identity.id.clone(),
            Thread {
                identity,
                basis: IdentityBasis::Native,
                aliases: Vec::new(),
                native_key,
                source: meta
                    .and_then(|meta| meta.source.clone())
                    .map_or(Basis::Unknown, Basis::Observed),
                initiator: meta
                    .and_then(|meta| meta.thread_source.clone())
                    .map_or(Basis::Unknown, Basis::Observed),
                purpose: Basis::Unknown,
                execution_environment: Basis::Observed("local".to_owned()),
                project: meta
                    .and_then(|meta| meta.project.clone())
                    .map_or(Basis::Unknown, Basis::Inferred),
                account: Basis::Unknown,
                evidence: meta_by_thread
                    .get(&native)
                    .map(|(_, evidence)| vec![*evidence])
                    .unwrap_or_default(),
            },
        );
    }

    let mut relationships = Vec::new();
    for (thread, (meta, evidence)) in &meta_by_thread {
        let relation = meta
            .parent_thread_id
            .as_deref()
            .map(|parent| (parent, RelationshipKind::Spawn))
            .or_else(|| {
                meta.forked_from_id.as_deref().map(|parent| (parent, RelationshipKind::Fork))
            });
        let Some((parent, kind)) = relation else { continue };
        let (Some(from), Some(to)) = (thread_ids.get(parent), thread_ids.get(thread)) else {
            continue;
        };
        relationships.push(Relationship {
            kind,
            from: from.clone(),
            to: to.clone(),
            confidence: Confidence::Proven,
            evidence: vec![*evidence],
        });
    }

    // Reserve per rollout after earlier observation vectors have been moved. A single
    // reserve for every row would allocate the 224-byte observation table while every
    // worker result still holds its own copy.
    let mut observations = Vec::new();
    let mut diagnostics = source_diagnostics(&manifest, &source_table);
    let (losses, mut gaps) = super::snapshot_losses(&manifest, "Codex rollout", |entry| {
        let id = &entry.source.as_ref()?.id;
        Some(EvidenceRef::new(source_index(&source_table, Some(id)), 0, 0))
    });
    diagnostics.extend(losses);
    for (thread, (meta, evidence)) in &meta_by_thread {
        if meta.parent().is_some_and(|parent| !thread_ids.contains_key(parent)) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ThreadOrphan,
                thread_ids.get(thread).cloned(),
                [*evidence],
                "Codex thread names a parent whose rollout was not discovered",
            ));
        }
    }
    let mut copied_regions = 0_u64;
    let mut limit_observations = Vec::new();
    let mut known_turns = KnownTurns::new();
    for rollout in &rollouts {
        let (is_root, file_thread, root_turns) = match rollout {
            DecodedRollout::Observed(observed) => (
                observed.metas.root == Some(true),
                observed.file_thread.as_str(),
                &observed.root_turns,
            ),
            DecodedRollout::Pending(source) => (
                source.metas.root == Some(true),
                source.strings.resolve(source.file_thread),
                &source.root_turns,
            ),
        };
        if is_root {
            known_turns
                .entry(file_thread.to_owned())
                .or_default()
                .extend(root_turns.iter().copied());
        }
    }
    // A root decides a child's prefix only when none of its rollouts holds lineage or a
    // nested spawn parent: a parent link (top-level, or nested in an old-format subagent's
    // `source`), a boundary or another thread's header means its rollout may lack turns
    // the child copied.
    let mut lineage_threads: HashSet<&str> = HashSet::new();
    for rollout in &rollouts {
        let (thread, metas) = match rollout {
            DecodedRollout::Observed(observed) => (observed.file_thread.as_str(), &observed.metas),
            DecodedRollout::Pending(source) => {
                (source.strings.resolve(source.file_thread), &source.metas)
            }
        };
        if metas.lineage || metas.spawn_parent {
            lineage_threads.insert(thread);
        }
    }
    let complete: HashSet<String> = known_turns
        .keys()
        .filter(|thread| !lineage_threads.contains(thread.as_str()))
        .cloned()
        .collect();
    // Only the counter-only complete roots that a child deciding its prefix by turns names
    // as its parent need their totals, and only counter-only rollouts still hold records.
    let inferring_parents: HashSet<&str> = rollouts
        .iter()
        .filter_map(|rollout| match rollout {
            DecodedRollout::Pending(source) if infers_prefix_turns(source) => {
                source.metas.own.as_deref().and_then(|(meta, _)| meta.parent())
            }
            DecodedRollout::Pending(_) | DecodedRollout::Observed(_) => None,
        })
        .filter(|parent| complete.contains(*parent))
        .collect();
    let mut parent_totals: HashMap<String, HashSet<TotalDigest>> = HashMap::new();
    for rollout in &rollouts {
        let DecodedRollout::Pending(source) = rollout else { continue };
        let thread = source.strings.resolve(source.file_thread);
        if source.metas.root == Some(true) && inferring_parents.contains(thread) {
            parent_totals.entry(thread.to_owned()).or_default().extend(counter_totals(source));
        }
    }
    let parents = ParentEvidence { turns: known_turns, totals: parent_totals, complete };
    for rollout in rollouts {
        match rollout {
            DecodedRollout::Observed(mut observed) => {
                let index = source_index(&source_table, observed.id.as_ref());
                stamp_refs(
                    &mut observed.observations,
                    &mut observed.limit_observations,
                    &mut observed.diagnostics,
                    &mut observed.gaps,
                    index,
                );
                copied_regions = copied_regions.saturating_add(observed.copied_regions);
                observations.reserve(observed.observations.len());
                observations.append(&mut observed.observations);
                limit_observations.append(&mut observed.limit_observations);
                diagnostics.append(&mut observed.diagnostics);
                gaps.append(&mut observed.gaps);
            }
            DecodedRollout::Pending(source) => {
                let index = source_index(&source_table, source.id.as_ref());
                let mut observed = observe_parsed_source(&source, &thread_ids, &parents)?;
                stamp_refs(
                    &mut observed.observations,
                    &mut observed.limit_observations,
                    &mut observed.diagnostics,
                    &mut observed.gaps,
                    index,
                );
                copied_regions = copied_regions.saturating_add(observed.copied_regions);
                observations.reserve(observed.observations.len());
                observations.append(&mut observed.observations);
                limit_observations.append(&mut observed.limit_observations);
                diagnostics.append(&mut observed.diagnostics);
                gaps.append(&mut observed.gaps);
            }
        }
    }
    observations.shrink_to_fit();

    let mut ledger = reconcile_with_capacity(
        ReconcileInput {
            threads: threads.into_values().collect(),
            relationships,
            requests: observations,
            limit_observations,
            gaps,
            diagnostics,
            source_table,
            ..ReconcileInput::default()
        },
        &LatestRevision,
        capacity,
    )?;
    ledger.coverage.copies = ledger.coverage.copies.saturating_add(copied_regions);
    ledger.diagnostics.retain(|diagnostic| diagnostic.code != DiagnosticCode::CopyWithoutOriginal);
    let threads = std::mem::take(&mut ledger.threads);
    let relationships = std::mem::take(&mut ledger.relationships);
    let limit_observations = std::mem::take(&mut ledger.limit_observations);
    let sources = manifest
        .entries
        .iter()
        .cloned()
        .map(|snapshot| SourceArtifact {
            dialect_version: snapshot
                .source
                .as_ref()
                .and_then(|source| source_versions.get(&source.id))
                .cloned()
                .map_or(Basis::Unknown, Basis::Observed),
            capability: SourceCapability::Supported,
            snapshot,
        })
        .collect();
    Ok(Ingested { manifest, sources, threads, relationships, ledger, limit_observations })
}

fn legacy_copied_evidence(source: &ParsedSource, known_turns: &KnownTurns) -> CopiedEvidence {
    let strings = &source.strings;
    let file_thread = source.file_thread;
    let mut active_thread = file_thread;
    let mut evidence = Vec::new();
    let mut occurrences = 0_u64;
    for record in &source.records {
        if active_thread != file_thread {
            occurrences = occurrences.saturating_add(record.skipped_before);
        }
        match &record.kind {
            RecordKind::SessionMeta { id } => {
                if let Some(thread_id) = *id {
                    active_thread = thread_id;
                }
            }
            RecordKind::TurnContext { turn_id, .. } if active_thread != file_thread => {
                let is_child_turn = turn_id
                    .is_some_and(|turn| is_unknown_turn(known_turns, strings, active_thread, turn));
                if is_child_turn {
                    active_thread = file_thread;
                }
            }
            RecordKind::ThreadSettingsApplied { .. } => {
                active_thread = file_thread;
            }
            RecordKind::TurnContext { .. }
            | RecordKind::UsageRecord(_)
            | RecordKind::Compacted(_)
            | RecordKind::TokenCount { .. } => {}
        }
        if active_thread != file_thread {
            occurrences = occurrences.saturating_add(1);
            evidence.push(record.evidence(0));
        }
    }
    if active_thread != file_thread {
        occurrences = occurrences.saturating_add(source.trailing_skipped);
    }
    CopiedEvidence { evidence, occurrences }
}

struct CopiedEvidence {
    evidence: Vec<EvidenceRef>,
    occurrences: u64,
}

#[derive(Clone, Copy)]
struct CounterObservation<'a> {
    role: ObservationRole,
    owner: Option<&'a str>,
    thread_ids: &'a BTreeMap<String, AnalyticalId>,
    context: Option<&'a TurnContext>,
    delta: Option<TokenMeasures>,
}

fn counter_observation(
    record: &RecordView,
    last: &CodexUsage,
    total: &CodexUsage,
    counter: CounterObservation<'_>,
) -> Result<RequestObservation, AdapterError> {
    let mut observation = RequestObservation::new(record.evidence);
    observation.role = counter.role;
    observation.owner = counter
        .owner
        .and_then(|owner| counter.thread_ids.get(owner))
        .cloned()
        .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    if let Some(thread_id) = counter.owner.and_then(|owner| counter.thread_ids.get(owner)) {
        observation.keys.push(
            COUNTER_KEY
                .key(vec![
                    KeyComponent::text(thread_id.to_string()),
                    KeyComponent::text(counter_signature(total)),
                ])?
                .derive()?,
        );
    }
    let mut usage = codex_usage(last)?;
    if let Some(delta) = counter.delta {
        usage = delta;
    }
    observation.usage = Some(usage.into());
    observation.timestamp = record.timestamp;
    if let Some(context) = counter.context {
        apply_context(&mut observation, context);
    }
    Ok(observation)
}

/// Emits one limit observation per reported window, skipping a snapshot identical to the
/// previous one in the same stream of this rollout: owner thread, limit and window.
/// The stream a limit observation collapses within: owner thread, limit name and window.
type LimitStream = (Option<AnalyticalId>, Option<String>, &'static str);

fn append_limits(
    record: &RecordView,
    limits: &Arc<RateLimits>,
    owner: Option<&str>,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    previous: &mut BTreeMap<LimitStream, Arc<RateLimits>>,
    observations: &mut Vec<ProviderLimitObservation>,
) {
    let owner_thread = owner.and_then(|owner| thread_ids.get(owner)).cloned();
    for window in &limits.windows {
        let stream = (owner_thread.clone(), limits.limit_name.clone(), *window);
        let repeated = previous.get(&stream).is_some_and(|last| last.native == limits.native);
        previous.insert(stream, Arc::clone(limits));
        if repeated {
            continue;
        }
        observations.push(ProviderLimitObservation {
            limit_name: limits.limit_name.as_deref().map(Name::new),
            window: Some(Name::new(window)),
            observed_at: record
                .timestamp
                .map_or(Basis::Unknown, |time| Basis::Observed(time.get())),
            owner_thread: owner_thread.clone(),
            owner_request: None,
            native: Name::new(&limits.native),
            evidence: record.evidence,
        });
    }
}

fn source_diagnostics(manifest: &SnapshotManifest, sources: &SourceTable) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut locations: BTreeMap<&str, Vec<&crate::sources::manifest::ManifestEntry>> =
        BTreeMap::new();
    for entry in &manifest.entries {
        locations.entry(&entry.locator).or_default().push(entry);
        if let Some(evidence) = &entry.first_malformed {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::MalformedLine,
                None,
                [*evidence],
                "a complete Codex rollout line is malformed",
            ));
        }
        if let (Some(source), Some(tail)) = (&entry.source, entry.cutoff.pending_tail) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::PendingTail,
                None,
                [EvidenceRef::new(
                    source_index(sources, Some(&source.id)),
                    tail.offset,
                    tail.length,
                )],
                "the incomplete final Codex rollout line is pending",
            ));
        }
    }
    for entries in locations.values().filter(|entries| entries.len() > 1) {
        let evidence = entries.iter().filter_map(|entry| {
            entry
                .source
                .as_ref()
                .map(|source| EvidenceRef::new(source_index(sources, Some(&source.id)), 0, 0))
        });
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::CodexRolloutDuplicateLocation,
                None,
                evidence,
                "the same Codex thread and rollout were found at multiple locations",
            )
            .with_occurrences(u64::try_from(entries.len()).unwrap_or(u64::MAX)),
        );
    }
    diagnostics
}

fn counter_signature(total: &CodexUsage) -> String {
    [
        total.input,
        total.cached_input,
        total.cache_write_input,
        total.output,
        total.reasoning_output,
        total.total,
    ]
    .map(|count| count.map_or_else(|| "?".to_owned(), |value| value.to_string()))
    .join(":")
}

/// A copy observation of the usage another thread's counter repeats in this rollout, keyed
/// by that thread's latest response when one is known.
fn copied_counter_observation(
    view: &RecordView,
    usage: &CodexUsage,
    owner: Option<&str>,
    response_id: Option<&str>,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    context: Option<&TurnContext>,
) -> Result<RequestObservation, AdapterError> {
    let mut observation = RequestObservation::new(view.evidence);
    observation.role = ObservationRole::Copy;
    observation.owner = owner
        .and_then(|owner| thread_ids.get(owner))
        .cloned()
        .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    observation.usage = Some(codex_usage(usage)?.into());
    if let Some(response_id) = response_id {
        observation.keys.push(
            RESPONSE_KEY
                .key(vec![KeyComponent::text(PROVIDER_NAMESPACE), KeyComponent::text(response_id)])?
                .derive()?,
        );
    }
    observation.timestamp = view.timestamp;
    if let Some(context) = context {
        apply_context(&mut observation, context);
    }
    Ok(observation)
}

fn usage_observation(
    record: &RecordView,
    payload: &UsagePayload<'_>,
    default_role: ObservationRole,
    file_thread: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    turns: &Turns,
) -> Result<RequestObservation, AdapterError> {
    let owner = payload.thread_id;
    let mut observation = RequestObservation::new(record.evidence);
    observation.role = if default_role == ObservationRole::Copy || owner != Some(file_thread) {
        ObservationRole::Copy
    } else {
        ObservationRole::Original
    };
    observation.owner = owner
        .and_then(|owner| thread_ids.get(owner))
        .cloned()
        .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    if let Some(response_id) = payload.response_id {
        observation.keys.push(
            RESPONSE_KEY
                .key(vec![KeyComponent::text(PROVIDER_NAMESPACE), KeyComponent::text(response_id)])?
                .derive()?,
        );
    }
    if let Some(usage) = &payload.usage {
        observation.usage = Some(codex_usage(usage)?.into());
    }
    observation.timestamp = record.timestamp;
    if let Some(context) = payload.turn.and_then(|turn| turns.get(&turn)) {
        apply_context(&mut observation, context);
    }
    Ok(observation)
}

fn apply_context(observation: &mut RequestObservation, context: &TurnContext) {
    observation.model = context.model.map(|name| ModelName { name, basis: ModelBasis::Requested });
    observation.effort = context.effort;
}

fn codex_usage(usage: &CodexUsage) -> Result<TokenMeasures, AdapterError> {
    let mut measures = normalize_input(
        InputSemantics::IncludesCacheRead,
        NativeInput {
            input: usage.input,
            cache_read: usage.cached_input,
            cache_write: usage.cache_write_input,
        },
    )?;
    measures.output = usage.output;
    measures.reasoning = usage.reasoning_output;
    Ok(measures)
}

fn thread_identity(native: &str) -> Result<StoredIdentity, AdapterError> {
    Ok(agent_thread_identity(Agent::Codex, native)?)
}

struct RolloutName {
    thread_id: String,
    rollout_id: String,
}

fn rollout_name(locator: &str) -> RolloutName {
    let name = PathBuf::from(locator)
        .file_name()
        .map_or_else(|| locator.to_owned(), |name| name.to_string_lossy().into_owned());
    let stem = Representation::ALL
        .into_iter()
        .find_map(|representation| name.strip_suffix(representation.suffix()))
        .unwrap_or(&name);
    if let Some((base, rollout)) = stem.rsplit_once('_') {
        let thread = base.get(base.len().saturating_sub(36)..).unwrap_or(base);
        return RolloutName { thread_id: thread.to_owned(), rollout_id: rollout.to_owned() };
    }
    let thread = stem.get(stem.len().saturating_sub(36)..).unwrap_or(stem).to_owned();
    RolloutName { rollout_id: thread.clone(), thread_id: thread }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use proptest::prelude::*;

    use super::{
        CodexUsage, CountReader, CounterRole, CounterScope, Interner, ParsedRecord, RateLimits,
        RecordKind, SourceDecoder, UsageMask, ingest_root, pack_usage,
    };
    use crate::ledger::diagnostics::DiagnosticCode;
    use crate::sources::evidence::EvidenceRef;
    use crate::sources::reader::{RawRecord, RecordDisposition};

    /// Decodes one line as the next line of `decoder`'s rollout.
    fn decode(decoder: &mut SourceDecoder, line: &str) -> RecordDisposition {
        let evidence = EvidenceRef::new(0, 0, u64::try_from(line.len()).unwrap());
        decoder.decode(&RawRecord { evidence: &evidence, bytes: line.as_bytes() })
    }

    #[test]
    fn observations_stay_within_the_admission_key_bound() {
        use std::collections::{BTreeMap, HashMap};

        use super::{
            CounterObservation, RecordView, TurnContext, Turns, UsagePayload, counter_observation,
            thread_identity, usage_observation,
        };
        use crate::ledger::admission::model::key_bound;
        use crate::ledger::names::Name;
        use crate::ledger::reconcile::ObservationRole;
        use crate::selection::Agent;

        let thread = "019f0000-0000-7000-8000-000000000001";
        let thread_ids = BTreeMap::from([(thread.to_owned(), thread_identity(thread).unwrap().id)]);
        let record = RecordView { evidence: EvidenceRef::new(0, 0, 1), timestamp: None };
        let context =
            TurnContext { model: Some(Name::new("gpt-test")), effort: Some(Name::new("high")) };
        let usage =
            CodexUsage { input: Some(3), output: Some(1), total: Some(4), ..CodexUsage::default() };
        let mut interner = Interner::default();
        let turn = interner.intern("turn");
        let turns: Turns = HashMap::from([(turn, context)]);
        let payload = UsagePayload {
            thread_id: Some(thread),
            response_id: Some("resp"),
            usage: Some(usage),
            turn: Some(turn),
        };
        let direct = usage_observation(
            &record,
            &payload,
            ObservationRole::Original,
            thread,
            &thread_ids,
            &turns,
        )
        .unwrap();
        let counter = counter_observation(
            &record,
            &usage,
            &usage,
            CounterObservation {
                role: ObservationRole::Original,
                owner: Some(thread),
                thread_ids: &thread_ids,
                context: Some(&context),
                delta: None,
            },
        )
        .unwrap();
        for observation in [direct, counter] {
            // Each key is a key-graph node, as is the artifact-local key a keyless
            // observation gets; a revision-invariant field could split it and add one more.
            let nodes =
                observation.keys.len().max(1) + usize::from(!observation.invariants.is_empty());
            assert_eq!(u64::try_from(nodes).unwrap(), key_bound(Agent::Codex), "{observation:?}");
        }
    }

    #[test]
    fn admission_does_not_charge_metadata_without_usage() {
        let budget = crate::ledger::admission::MemoryAdmission::unlimited()
            .with_row_ceiling(crate::ledger::capacity::ObservationCapacity::from_rows(0));
        let evidence = EvidenceRef::new(0, 0, 1);
        let mut decoder = SourceDecoder::new("one");
        for line in [
            r#"{"type":"compacted","payload":{}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":null,"rate_limits":{}}}"#,
        ] {
            let raw = RawRecord { evidence: &evidence, bytes: line.as_bytes() };
            assert_eq!(
                decoder.decode_with_admission(&raw, Some(&budget)),
                RecordDisposition::Decoded
            );
        }
        assert!(!budget.stopped());
    }

    #[test]
    fn admission_charges_pending_usage_and_stops_other_decoders() {
        for line in [
            r#"{"type":"token_usage_record","payload":{"usage":{"input_tokens":3}}}"#,
            r#"{"type":"compacted","payload":{"latest_token_usage_record":{"usage":{"input_tokens":3}}}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":3}}}}"#,
        ] {
            let budget = crate::ledger::admission::MemoryAdmission::unlimited()
                .with_row_ceiling(crate::ledger::capacity::ObservationCapacity::from_rows(1));
            let evidence = EvidenceRef::new(0, 0, 1);
            let raw = RawRecord { evidence: &evidence, bytes: line.as_bytes() };
            let mut first = SourceDecoder::new("one");
            let mut second = SourceDecoder::new("two");
            assert_eq!(
                first.decode_with_admission(&raw, Some(&budget)),
                RecordDisposition::Decoded
            );
            assert_eq!(second.decode_with_admission(&raw, Some(&budget)), RecordDisposition::Stop);
            assert_eq!(first.records.len(), 1);
            assert!(second.records.is_empty());
            let skipped = RawRecord { evidence: &evidence, bytes: b"{}" };
            assert_eq!(
                first.decode_with_admission(&skipped, Some(&budget)),
                RecordDisposition::Stop
            );
        }
    }

    #[test]
    fn a_decoded_record_is_compact() {
        assert_eq!(size_of::<ParsedRecord>(), 80);
        assert_eq!(size_of::<RecordKind>(), 24);
    }

    #[test]
    fn observation_slots_count_usage_bearing_records_only() {
        let mut decoder = SourceDecoder::new("thread");
        assert_eq!(
            decode(&mut decoder, r#"{"type":"session_meta","payload":{"id":"thread"}}"#),
            RecordDisposition::Decoded
        );
        assert_eq!(
            decode(&mut decoder, r#"{"type":"turn_context","payload":{"turn_id":"t1"}}"#),
            RecordDisposition::Decoded
        );
        assert_eq!(
            decode(
                &mut decoder,
                r#"{"type":"token_usage_record","payload":{"response_id":"r1","usage":{"input_tokens":1}}}"#
            ),
            RecordDisposition::Decoded
        );
        assert_eq!(
            decode(&mut decoder, r#"{"type":"event_msg","payload":{"type":"token_count"}}"#),
            RecordDisposition::Decoded
        );
        assert_eq!(decoder.records.len(), 4);
        assert_eq!(decoder.observation_slots, 2);

        let parsed = decoder.finish();
        assert_eq!(parsed.observation_slots, 2);
        assert_eq!(parsed.root_turns.len(), 1);
    }

    #[test]
    fn skipped_record_payloads_are_not_retained() {
        let payload = "x".repeat(2 * 1024 * 1024);
        let line = format!(r#"{{"type":"response_item","payload":{{"content":"{payload}"}}}}"#);
        let mut decoder = SourceDecoder::new("thread");

        assert_eq!(decode(&mut decoder, &line), RecordDisposition::Skipped);
        assert!(decoder.records.is_empty());
        assert_eq!(decoder.skipped, 1);
    }

    #[test]
    fn legacy_copied_history_counts_skipped_spans_without_retaining_their_evidence() {
        let home = tempfile::tempdir().unwrap();
        let sessions = home.path().join("sessions/2026/09/16");
        fs::create_dir_all(&sessions).unwrap();
        let parent = "00000000-0000-7000-8000-000000000001";
        let child = "00000000-0000-7000-8000-000000000002";
        let rollout = sessions.join(format!("rollout-2026-09-16T12-00-00-{child}.jsonl"));
        fs::write(
            rollout,
            format!(
                concat!(
                    "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{child}\",\"parent_thread_id\":\"{parent}\"}}}}\n",
                    "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{parent}\"}}}}\n",
                    "{{\"type\":\"response_item\",\"payload\":{{\"content\":\"inside copied history\"}}}}\n",
                    "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\"}}}}\n",
                    "{{\"type\":\"response_item\",\"payload\":{{\"content\":\"trailing copied history\"}}}}\n"
                ),
                child = child,
                parent = parent,
            ),
        )
        .unwrap();

        let ingested = ingest_root(home.path()).unwrap();
        let copied = ingested
            .ledger
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagnosticCode::CodexCopiedHistoryInferred)
            .expect("legacy copied history emits a diagnostic");

        assert_eq!(copied.occurrences, 4);
        assert_eq!(copied.evidence.len(), 2, "only relevant copied records retain evidence");
        assert_eq!(ingested.manifest.entries[0].counters.skipped, 2);
    }

    #[test]
    fn codex_home_discovers_only_rollout_directories() {
        let home = tempfile::tempdir().unwrap();
        let sessions = home.path().join("sessions/2026/09/16");
        fs::create_dir_all(&sessions).unwrap();
        let thread = "00000000-0000-7000-8000-000000000001";
        let rollout = sessions.join(format!("rollout-2026-09-16T12-00-00-{thread}.jsonl"));
        fs::write(rollout, format!(r#"{{"type":"session_meta","payload":{{"id":"{thread}"}}}}"#))
            .unwrap();
        fs::write(
            home.path().join("unrelated.jsonl"),
            r#"{"type":"response_item","payload":{"content":"not a rollout"}}"#,
        )
        .unwrap();

        let ingested = ingest_root(home.path()).unwrap();

        assert_eq!(ingested.manifest.entries.len(), 1);
    }

    #[test]
    fn the_prefilter_skips_valid_content_and_still_reports_malformed_lines() {
        let decode = |line: &str| {
            let mut decoder = SourceDecoder::new("thread");
            let disposition = decode(&mut decoder, line);
            (disposition, decoder.records.len(), decoder.skipped)
        };

        assert_eq!(
            decode(r#"{"type":"response_item","payload":{"content":"text"}}"#),
            (RecordDisposition::Skipped, 0, 1)
        );
        assert_eq!(
            decode(r#"{"type":"response_item","payload":{"content":"#),
            (RecordDisposition::Malformed, 0, 0)
        );
        assert_eq!(
            decode(r#"{"type":"response_item","payload":{"content":"a \"token_count\" mention"}}"#),
            (RecordDisposition::Skipped, 0, 1)
        );
        assert_eq!(
            decode(r#"{"type":"turn_context","payload":{"turn_id":"t1"}}"#),
            (RecordDisposition::Decoded, 1, 0)
        );
        assert_eq!(
            decode(r#"{"type":"turn_context","payload":{"turn_id":"t1","x":"\ud800"}}"#),
            (RecordDisposition::Malformed, 0, 0)
        );
        assert_eq!(
            decode(r#"{"type":"response_item","payload":{"content":"\ud800"}}"#),
            (RecordDisposition::Malformed, 0, 0)
        );
        assert_eq!(
            decode(r#"{"type":"response_item","payload":{"size":1e400}}"#),
            (RecordDisposition::Malformed, 0, 0)
        );
    }

    #[test]
    fn relevant_records_keep_only_accounting_fields() {
        let payload = "x".repeat(2 * 1024 * 1024);
        let line = serde_json::json!({
            "type": "compacted",
            "timestamp": "2026-09-16T12:00:00Z",
            "payload": {
                "latest_token_usage_record": {
                    "response_id": "response-one",
                    "usage": {"input_tokens": 3, "output_tokens": 10}
                },
                "replacement_history": [{"content": payload}]
            }
        })
        .to_string();
        let mut decoder = SourceDecoder::new("thread");

        assert_eq!(decode(&mut decoder, &line), RecordDisposition::Decoded);
        let latest = match decoder.records.first().map(|record| &record.kind) {
            Some(RecordKind::Compacted(Some(latest))) => Some(*latest),
            _ => None,
        }
        .expect("a compacted record keeps its latest usage record");
        let strings = &decoder.interner.strings;
        assert_eq!(latest.response_id.map(|id| strings.resolve(id)), Some("response-one"));
        let [usage, none] = CountReader::new(&decoder.counts).record(&decoder.records[0].kind);
        assert_eq!(usage.and_then(|usage| usage.input), Some(3));
        assert_eq!(usage.and_then(|usage| usage.output), Some(10));
        assert_eq!(none, None);
        assert!(strings.text.len() < 64 && decoder.counts.len() < 8, "no content is kept");
    }

    #[test]
    fn identical_consecutive_rate_limits_share_one_snapshot() {
        let mut decoder = SourceDecoder::new("thread");
        let mut limits = |used: f64| -> Arc<RateLimits> {
            let line = serde_json::json!({
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "rate_limits": {"limit_id": "codex", "primary": {"used_percent": used}}
                }
            })
            .to_string();
            assert_eq!(decode(&mut decoder, &line), RecordDisposition::Decoded);
            match decoder.records.last().map(|record| &record.kind) {
                Some(RecordKind::TokenCount { limits: Some(limits), .. }) => {
                    Some(Arc::clone(limits))
                }
                _ => None,
            }
            .expect("a token count with rate limits keeps them")
        };
        let first = limits(1.0);
        let repeat = limits(1.0);
        let changed = limits(2.0);

        assert!(Arc::ptr_eq(&first, &repeat));
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(first.windows, ["primary"]);
        assert_eq!(first.limit_name.as_deref(), Some("codex"));
    }

    #[test]
    fn interned_strings_share_symbols_exactly_when_their_text_matches() {
        let mut interner = Interner::default();
        let texts = ["thread", "", "turn-1", "thread", "", "turn-2", "turn-1", "t\u{e9}"];
        let symbols: Vec<_> = texts.iter().map(|text| interner.intern(text)).collect();
        let strings = interner.finish();
        for (left, left_text) in symbols.iter().zip(texts) {
            assert_eq!(strings.resolve(*left), left_text);
            for (right, right_text) in symbols.iter().zip(texts) {
                assert_eq!(left == right, left_text == right_text);
            }
        }
    }

    /// A Codex usage object with `total_tokens` set to input plus output, plus `extra`.
    fn usage_json(input: u64, output: u64, reasoning: u64, extra: u64) -> serde_json::Value {
        serde_json::json!({
            "input_tokens": input, "cached_input_tokens": 0, "cache_write_input_tokens": 0,
            "output_tokens": output, "reasoning_output_tokens": reasoning,
            "total_tokens": input + output + extra
        })
    }

    fn record_line(usage: &serde_json::Value) -> String {
        serde_json::json!({"type": "token_usage_record", "payload": {
            "thread_id": "thread", "response_id": "response", "usage": usage
        }})
        .to_string()
    }

    fn counter_line(total: &serde_json::Value, last: &serde_json::Value) -> String {
        serde_json::json!({"type": "event_msg", "payload": {"type": "token_count", "info": {
            "total_token_usage": total, "last_token_usage": last
        }}})
        .to_string()
    }

    const LIMITS_ONLY: &str =
        r#"{"type":"event_msg","payload":{"type":"token_count","info":null}}"#;

    /// Decodes `lines` as the rollout of thread `thread`.
    fn parse(lines: &[String]) -> super::ParsedSource {
        let mut decoder = SourceDecoder::new("thread");
        for line in lines {
            assert_eq!(decode(&mut decoder, line), RecordDisposition::Decoded, "{line}");
        }
        decoder.finish()
    }

    /// The scope and the role of each `token_count`, in record order.
    fn roles(lines: &[String]) -> (CounterScope, Vec<CounterRole>) {
        let parsed = parse(lines);
        let roles = parsed
            .records
            .iter()
            .filter_map(|record| match record.kind {
                RecordKind::TokenCount { role, .. } => Some(role),
                RecordKind::SessionMeta { .. }
                | RecordKind::TurnContext { .. }
                | RecordKind::UsageRecord(_)
                | RecordKind::Compacted(_)
                | RecordKind::ThreadSettingsApplied { .. } => None,
            })
            .collect();
        (parsed.counter_scope, roles)
    }

    #[test]
    fn counter_roles_follow_adjacent_usage_events_with_exactly_equal_usage() {
        use CounterRole::{Counted, Covered, Twin};
        let one = usage_json(1_000, 100, 10, 0);
        let two = usage_json(3_000, 300, 30, 0);
        let both = usage_json(4_000, 400, 40, 0);
        let estimate = usage_json(0, 0, 0, 3_200);
        let record = record_line(&one);
        let twin = counter_line(&one, &one);
        let fork = r#"{"type":"session_meta","payload":{"id":"thread","forked_from_id":"parent"}}"#;
        let cases = [
            (
                "no usage record",
                vec![twin.clone(), counter_line(&two, &usage_json(2_000, 200, 20, 0))],
                (CounterScope::All, vec![Counted, Counted]),
            ),
            (
                "Codex's order",
                vec![record.clone(), twin.clone()],
                (CounterScope::Direct, vec![Twin]),
            ),
            (
                "the reverse order",
                vec![twin.clone(), record.clone()],
                (CounterScope::Direct, vec![Twin]),
            ),
            (
                "a record is the twin of the first counter to reach it",
                vec![
                    twin.clone(),
                    record.clone(),
                    counter_line(&usage_json(2_000, 200, 20, 0), &one),
                ],
                (CounterScope::Mixed, vec![Twin, Counted]),
            ),
            (
                "no pair spans a turn_context",
                vec![
                    twin.clone(),
                    r#"{"type":"turn_context","payload":{"turn_id":"t2"}}"#.to_owned(),
                    record.clone(),
                    counter_line(&usage_json(2_000, 200, 20, 0), &one),
                ],
                (CounterScope::Mixed, vec![Counted, Twin]),
            ),
            (
                "equal consecutive responses in the reverse order",
                vec![
                    twin.clone(),
                    record.clone(),
                    counter_line(&usage_json(2_000, 200, 20, 0), &one),
                    record.clone(),
                ],
                (CounterScope::Direct, vec![Twin, Twin]),
            ),
            (
                "another reasoning count",
                vec![record.clone(), counter_line(&one, &usage_json(1_000, 100, 11, 0))],
                (CounterScope::Mixed, vec![Counted]),
            ),
            (
                "another total_tokens",
                vec![record.clone(), counter_line(&one, &usage_json(1_000, 100, 10, 1))],
                (CounterScope::Mixed, vec![Counted]),
            ),
            (
                "a refresh and a limits-only counter after a twin",
                vec![record.clone(), twin.clone(), twin.clone(), LIMITS_ONLY.to_owned()],
                (CounterScope::Direct, vec![Twin, Covered, Covered]),
            ),
            (
                "estimates follow the usage event before them",
                vec![
                    twin.clone(),
                    counter_line(&one, &estimate),
                    record_line(&two),
                    counter_line(&both, &two),
                    counter_line(&both, &estimate),
                ],
                (CounterScope::Mixed, vec![Counted, Counted, Twin, Covered]),
            ),
            (
                "a leading counter follows the first usage event",
                vec![LIMITS_ONLY.to_owned(), record.clone(), twin.clone()],
                (CounterScope::Direct, vec![Covered, Twin]),
            ),
            (
                "a rollout that is not a root leaves its counters to its usage records",
                vec![fork.to_owned(), counter_line(&two, &two), record, twin],
                (CounterScope::Direct, vec![Counted, Counted]),
            ),
        ];
        for (name, lines, expected) in cases {
            assert_eq!(roles(&lines), expected, "{name}");
        }
    }

    #[test]
    fn a_root_rollout_names_no_other_thread_parent_fork_or_history_boundary() {
        let meta = |fields: &str| format!(r#"{{"type":"session_meta","payload":{{{fields}}}}}"#);
        let settings = |thread: &str| {
            format!(
                r#"{{"type":"event_msg","payload":{{"type":"thread_settings_applied","thread_id":"{thread}"}}}}"#
            )
        };
        let record = |thread: &str| {
            format!(
                r#"{{"type":"token_usage_record","payload":{{"thread_id":"{thread}","usage":{{"input_tokens":1}}}}}}"#
            )
        };
        let cases = [
            ("no session_meta", vec![record("thread")], true),
            (
                "its own session_meta and settings",
                vec![meta(r#""id":"thread""#), settings("thread")],
                true,
            ),
            (
                "a session_meta of another thread",
                vec![meta(r#""id":"thread""#), meta(r#""id":"other""#)],
                false,
            ),
            ("a session_meta without an ID", vec![meta(r#""cwd":"/work""#)], false),
            ("a parent", vec![meta(r#""id":"thread","parent_thread_id":"other""#)], false),
            ("a fork origin", vec![meta(r#""id":"thread","forked_from_id":"other""#)], false),
            (
                "a history boundary on a later session_meta",
                vec![
                    meta(r#""id":"thread""#),
                    meta(r#""id":"thread","subagent_history_start_ordinal":0"#),
                ],
                false,
            ),
            (
                "settings naming another thread",
                vec![meta(r#""id":"thread""#), settings("other")],
                false,
            ),
            ("a usage record of another thread", vec![record("other")], false),
        ];
        for (name, lines, root) in cases {
            let parsed = parse(&lines);
            assert_eq!(
                super::is_root_rollout(&parsed.metas, &parsed.records, parsed.file_thread),
                root,
                "{name}"
            );
        }
    }

    #[test]
    fn only_a_rollout_without_usage_records_waits_for_normalization() {
        // A rollout waits for every rollout to decode exactly when its counter scope is
        // `All` (`decode_rollout`), so a root rollout with usage records never waits.
        let one = usage_json(1_000, 100, 0, 0);
        let two = usage_json(3_000, 300, 0, 0);
        let own_meta =
            r#"{"type":"session_meta","payload":{"id":"thread","parent_thread_id":"parent"}}"#;
        let parent_meta = r#"{"type":"session_meta","payload":{"id":"parent"}}"#;
        let parent_turn = r#"{"type":"turn_context","payload":{"turn_id":"p1"}}"#;
        let legacy = counter_line(&one, &one);
        let recorded = [
            record_line(&usage_json(2_000, 200, 0, 0)),
            counter_line(&two, &usage_json(2_000, 200, 0, 0)),
        ];
        let lines = |prefix: &[&str], counters: &[String]| -> Vec<String> {
            prefix.iter().map(|line| (*line).to_owned()).chain(counters.iter().cloned()).collect()
        };
        let cases = [
            ("counter-only", lines(&[], std::slice::from_ref(&legacy)), CounterScope::All),
            ("every counter a twin", lines(&[], &recorded), CounterScope::Direct),
            (
                "a resumed root session",
                lines(&[], &[[legacy.clone()].as_slice(), &recorded].concat()),
                CounterScope::Mixed,
            ),
            (
                "a resumed legacy subagent",
                lines(
                    &[own_meta, parent_meta, parent_turn],
                    &[[legacy].as_slice(), &recorded].concat(),
                ),
                CounterScope::Direct,
            ),
        ];
        for (name, lines, scope) in cases {
            assert_eq!(parse(&lines).counter_scope, scope, "{name}");
        }
    }

    fn usage() -> impl Strategy<Value = Option<CodexUsage>> {
        let count = prop_oneof![
            Just(None),
            Just(Some(0)),
            Just(Some(u64::MAX)),
            (0_u64..1 << 40).prop_map(Some),
            any::<u64>().prop_map(Some),
        ];
        prop::option::of(prop::array::uniform6(count).prop_map(CodexUsage::from_counts))
    }

    proptest! {
        #[test]
        fn packed_counts_read_back_in_record_order(
            records in prop::collection::vec((usage(), usage(), any::<bool>()), 0..20),
        ) {
            let mut counts = Vec::new();
            let kinds: Vec<RecordKind> = records
                .iter()
                .map(|(first, second, token_count)| {
                    let mut mask = UsageMask::default();
                    pack_usage(&mut counts, &mut mask, 0, *first);
                    if *token_count {
                        pack_usage(&mut counts, &mut mask, 1, *second);
                        RecordKind::TokenCount { usage: mask, limits: None, role: CounterRole::Counted }
                    } else {
                        RecordKind::UsageRecord(super::UsageRecord {
                            thread_id: None,
                            response_id: None,
                            turn: None,
                            usage: mask,
                        })
                    }
                })
                .collect();
            let mut reader = CountReader::new(&counts);
            for ((first, second, token_count), kind) in records.iter().zip(&kinds) {
                let expected = [*first, if *token_count { *second } else { None }];
                prop_assert_eq!(reader.record(kind), expected);
            }
            prop_assert_eq!(reader.at, counts.len());
        }
    }
}
