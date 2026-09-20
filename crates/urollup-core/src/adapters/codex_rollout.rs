//! Codex rollout adapter (`codex-rollout`).
//!
//! Decoding keeps memory proportional to relevant records, not to log bytes:
//!
//! - A line the byte prefilter rules out is only validated. Any other line is read by a
//!   typed first pass, which borrows its strings and builds no JSON document apart from a
//!   `rate_limits` object.
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

use serde_json::{Map, Value};

use self::line::{EventType, Line, Payload, RecordType, UsageFields};
use super::{AdapterError, Ingested};
use crate::ledger::counters::{CounterEvent, RunningTotal};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{
    Basis, CompactTimestamp, Confidence, ModelBasis, ModelName, ProviderLimitObservation,
    Relationship, RelationshipKind, SourceArtifact, SourceCapability, Thread,
};
use crate::ledger::identity::{AnalyticalId, IdPrefix, KeyComponent, StoredIdentity, sha256_128};
use crate::ledger::names::Name;
use crate::ledger::reconcile::{
    LatestRevision, ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation, reconcile,
};
use crate::ledger::scope::{ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec};
use crate::ledger::tokens::{InputSemantics, NativeInput, TokenMeasures, normalize_input};
use crate::selection::{Agent, agent_thread_identity};
use crate::sources::admission::Admission;
use crate::sources::decode::{parse_timestamp, validate_record};
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::{ManifestEntry, SnapshotManifest};
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
/// count mask and an 8-byte shared rate-limit pointer).
const _: () =
    assert!(size_of::<ParsedRecord>() <= 80, "a decoded Codex record outgrew its size budget");

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
    },
    ThreadSettingsApplied {
        thread_id: Option<Sym>,
    },
}

impl ParsedRecord {
    fn evidence(&self, source: &AnalyticalId) -> EvidenceRef {
        EvidenceRef { source: source.clone(), offset: self.offset, length: self.length }
    }
}

/// A `token_usage_record` payload, or the latest one a `compacted` record copies, whose
/// usage is in count slot 0.
#[derive(Clone, Copy)]
struct UsageRecord {
    thread_id: Option<Sym>,
    response_id: Option<Sym>,
    root_turn_id: Option<Sym>,
    usage: UsageMask,
}

impl UsageRecord {
    fn payload<'a>(&self, strings: &'a Strings, usage: Option<CodexUsage>) -> UsagePayload<'a> {
        UsagePayload {
            thread_id: self.thread_id.map(|thread| strings.resolve(thread)),
            response_id: self.response_id.map(|response| strings.resolve(response)),
            usage,
            root_turn_id: self.root_turn_id,
        }
    }
}

/// A usage record with its strings and counts read back.
struct UsagePayload<'a> {
    thread_id: Option<&'a str>,
    response_id: Option<&'a str>,
    usage: Option<CodexUsage>,
    root_turn_id: Option<Sym>,
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
    subagent_history_start_ordinal: Option<u64>,
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
    if let (true, Some(root)) = (missing_is_error, discovery.missing_roots.first()) {
        return Err(AdapterError::MissingRoot(root.clone()));
    }
    if let Some(unreadable) = discovery.unreadable.first() {
        return Err(AdapterError::UnreadablePath {
            path: unreadable.path.clone(),
            kind: unreadable.kind,
        });
    }

    let admission = Admission::new(crate::ledger::reconcile::MAX_OBSERVATIONS);
    let decoded = try_read_in_parallel(
        &discovery.sources,
        workers,
        |source| source_weight(&source.files),
        |source| decode_source(source, &admission),
    );
    if admission.stopped() {
        return Err(admission.error().into());
    }
    let decoded = decoded?;
    let (entries, parsed_sources): (Vec<_>, Vec<_>) = decoded.into_iter().unzip();
    let manifest = SnapshotManifest { entries, skipped_links: discovery.skipped_links };

    normalize(parsed_sources, manifest)
}

/// Reads one rollout, independently of every other source.
fn decode_source(
    source: &DiscoveredSource,
    admission: &Admission,
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
    Ok((entry, decoder.finish()))
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
        }
    }

    fn finish(self) -> ParsedSource {
        let Self { id, file_thread, interner, mut counts, mut records, metas, skipped, .. } = self;
        // Decoding grows the buffers by doubling; release the unused tails before the
        // records wait for the other rollouts.
        counts.shrink_to_fit();
        records.shrink_to_fit();
        ParsedSource {
            id,
            file_thread,
            strings: interner.finish(),
            counts,
            records,
            metas,
            trailing_skipped: skipped,
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
        admission: Option<&Admission>,
    ) -> RecordDisposition {
        if admission.is_some_and(Admission::stopped) {
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
        if request_bearing && admission.is_some_and(|budget| !budget.reserve()) {
            return RecordDisposition::Stop;
        }
        self.id.get_or_insert_with(|| raw.evidence.source.clone());
        self.records.push(ParsedRecord {
            offset: raw.evidence.offset,
            length: raw.evidence.length,
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
            RecordType::TokenUsageRecord => {
                Some(RecordKind::UsageRecord(self.usage_record(&payload.usage_record)))
            }
            RecordType::Compacted => Some(RecordKind::Compacted(
                payload.latest_token_usage_record.as_ref().map(|latest| self.usage_record(latest)),
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
                    })
                }
                EventType::Other => None,
            },
            RecordType::Other => None,
        }
    }

    fn usage_record(&mut self, fields: &UsageFields<'_>) -> UsageRecord {
        let mut usage = UsageMask::default();
        pack_usage(&mut self.counts, &mut usage, 0, fields.usage);
        UsageRecord {
            thread_id: self.interner.intern_some(fields.thread_id.as_deref()),
            response_id: self.interner.intern_some(fields.response_id.as_deref()),
            root_turn_id: self.interner.intern_some(fields.root_turn_id.as_deref()),
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
            metas.own = Some(Box::new((meta, evidence.clone())));
        }
        RecordKind::SessionMeta { id }
    }
}

/// A token count's `rate_limits` object, sharing the previous snapshot when identical.
fn rate_limits(
    rate_limits: Option<Map<String, Value>>,
    last: &mut Option<Arc<RateLimits>>,
) -> Option<Arc<RateLimits>> {
    let object = rate_limits?;
    let sorted: BTreeMap<&String, &Value> = object.iter().collect();
    let native = serde_json::to_string(&sorted).unwrap_or_default();
    if let Some(previous) = last.as_ref().filter(|previous| previous.native == native) {
        return Some(Arc::clone(previous));
    }
    let name = |key: &str| object.get(key).and_then(Value::as_str);
    let limits = Arc::new(RateLimits {
        limit_name: name("limit_id").or_else(|| name("limit_name")).map(str::to_owned),
        windows: ["primary", "secondary"]
            .into_iter()
            .filter(|window| object.get(*window).is_some_and(|value| !value.is_null()))
            .collect(),
        native,
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

fn normalize(
    sources: Vec<ParsedSource>,
    manifest: SnapshotManifest,
) -> Result<Ingested, AdapterError> {
    let mut source_versions: BTreeMap<AnalyticalId, String> = BTreeMap::new();
    for source in &sources {
        if let (Some(id), Some(version)) = (&source.id, &source.metas.cli_version) {
            source_versions.entry(id.clone()).or_insert_with(|| version.clone());
        }
    }
    let mut native_threads: BTreeSet<String> = BTreeSet::new();
    // Session metadata is copied out, one per thread, so each rollout's records can be
    // freed as soon as its observations are built.
    let mut meta_by_thread: BTreeMap<String, (SessionMeta, EvidenceRef)> = BTreeMap::new();
    for source in &sources {
        let file_thread = source.strings.resolve(source.file_thread);
        native_threads.insert(file_thread.to_owned());
        if let Some(own) = &source.metas.own {
            meta_by_thread.entry(file_thread.to_owned()).or_insert_with(|| (**own).clone());
        }
    }

    let mut thread_ids = BTreeMap::new();
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
                    .map(|(_, evidence)| vec![evidence.clone()])
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
            evidence: vec![evidence.clone()],
        });
    }

    let mut observations =
        Vec::with_capacity(sources.iter().map(|source| source.records.len()).sum());
    let mut diagnostics = source_diagnostics(&manifest);
    for (thread, (meta, evidence)) in &meta_by_thread {
        if meta.parent().is_some_and(|parent| !thread_ids.contains_key(parent)) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ThreadOrphan,
                thread_ids.get(thread).cloned(),
                [evidence.clone()],
                "Codex thread names a parent whose rollout was not discovered",
            ));
        }
    }
    let mut copied_regions = 0_u64;
    let mut limit_observations = Vec::new();
    let mut known_turns = KnownTurns::new();
    for source in &sources {
        if source.metas.root == Some(true) {
            let strings = &source.strings;
            let turns =
                known_turns.entry(strings.resolve(source.file_thread).to_owned()).or_default();
            turns.extend(source.records.iter().filter_map(|record| match record.kind {
                RecordKind::TurnContext { turn_id, .. } => {
                    turn_id.map(|turn| turn_digest(strings.resolve(turn)))
                }
                RecordKind::SessionMeta { .. }
                | RecordKind::UsageRecord(_)
                | RecordKind::Compacted(_)
                | RecordKind::TokenCount { .. }
                | RecordKind::ThreadSettingsApplied { .. } => None,
            }));
        }
    }
    for source in sources {
        // A rollout without records has no metadata and builds nothing.
        let Some(source_id) = source.id.as_ref() else { continue };
        let strings = &source.strings;
        let file_thread = source.file_thread;
        let file_thread_text = strings.resolve(file_thread);
        let has_direct =
            source.records.iter().any(|record| matches!(record.kind, RecordKind::UsageRecord(_)));
        let own_meta = source.metas.own.as_deref().map(|(meta, _)| meta);
        let parent_thread = own_meta.and_then(SessionMeta::parent);
        let native_boundary = own_meta.and_then(|meta| meta.subagent_history_start_ordinal);
        let has_foreign_meta = source.metas.foreign;
        if parent_thread.is_some() && has_foreign_meta && (!has_direct || native_boundary.is_some())
        {
            copied_regions = copied_regions.saturating_add(1);
            if !has_direct && native_boundary.is_none() {
                let copied = legacy_copied_evidence(&source, source_id, &known_turns);
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
        let mut turns = Turns::new();
        let mut current_turn: Option<Sym> = None;
        let mut last_response_by_thread: HashMap<Sym, Sym> = HashMap::new();
        let mut inherited_total = None;
        let mut counter = None;
        let mut previous_limits = BTreeMap::new();
        let mut counts = CountReader::new(&source.counts);
        for record in &source.records {
            // Counts are packed in record order, so every record reads its own first.
            let [first_usage, second_usage] = counts.record(&record.kind);
            let view =
                RecordView { evidence: record.evidence(source_id), timestamp: record.timestamp };
            if native_boundary
                .is_some_and(|boundary| record.ordinal.is_some_and(|ordinal| ordinal >= boundary))
            {
                active_thread = file_thread;
            }
            match &record.kind {
                RecordKind::SessionMeta { id } => {
                    if let Some(thread_id) = *id {
                        active_thread = thread_id;
                    }
                }
                RecordKind::TurnContext { turn_id, model, effort } => {
                    if !has_direct
                        && active_thread != file_thread
                        && turn_id.is_some_and(|turn| {
                            is_unknown_turn(&known_turns, strings, active_thread, turn)
                        })
                    {
                        active_thread = file_thread;
                    }
                    if let Some(turn_id) = *turn_id {
                        turns.insert(turn_id, TurnContext { model: *model, effort: *effort });
                    }
                    current_turn = *turn_id;
                }
                RecordKind::ThreadSettingsApplied { thread_id } => {
                    active_thread = thread_id.unwrap_or(file_thread);
                }
                RecordKind::UsageRecord(usage_record) => {
                    let payload = usage_record.payload(strings, first_usage);
                    let observation =
                        direct_observation(&view, &payload, file_thread_text, &thread_ids, &turns)?;
                    if let Some(response_id) = usage_record.response_id {
                        let owner = usage_record.thread_id.unwrap_or(file_thread);
                        last_response_by_thread.insert(owner, response_id);
                    }
                    observations.push(observation);
                }
                RecordKind::Compacted(latest) => {
                    if let Some(latest) = latest {
                        observations.push(usage_observation(
                            &view,
                            &latest.payload(strings, first_usage),
                            ObservationRole::Copy,
                            file_thread_text,
                            &thread_ids,
                            &turns,
                        )?);
                    }
                }
                RecordKind::TokenCount { limits, .. } => {
                    let (total, last) = (first_usage, second_usage);
                    if let Some(limits) = limits {
                        append_limits(
                            &view,
                            limits,
                            strings.resolve(active_thread),
                            &thread_ids,
                            &mut previous_limits,
                            &mut limit_observations,
                        );
                    }
                    if has_direct && active_thread != file_thread {
                        if let Some(usage) = &last {
                            let mut observation = RequestObservation::new(view.evidence.clone());
                            observation.role = ObservationRole::Copy;
                            observation.owner = thread_ids
                                .get(strings.resolve(active_thread))
                                .cloned()
                                .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
                            observation.usage = Some(codex_usage(usage)?.into());
                            if let Some(response_id) = last_response_by_thread.get(&active_thread) {
                                observation.keys.push(
                                    RESPONSE_KEY
                                        .key(vec![
                                            KeyComponent::text(PROVIDER_NAMESPACE),
                                            KeyComponent::text(strings.resolve(*response_id)),
                                        ])?
                                        .derive()?,
                                );
                            }
                            observation.timestamp = view.timestamp;
                            if let Some(context) = current_turn.and_then(|turn| turns.get(&turn)) {
                                apply_context(&mut observation, context);
                            }
                            observations.push(observation);
                        }
                    } else if !has_direct {
                        let Some(total) = &total else { continue };
                        let total_usage = codex_usage(total)?;
                        let last = last.as_ref();
                        if active_thread != file_thread {
                            inherited_total = Some(total_usage);
                            if let Some(last) = last {
                                observations.push(counter_observation(
                                    &view,
                                    last,
                                    total,
                                    CounterObservation {
                                        role: ObservationRole::Copy,
                                        owner: strings.resolve(active_thread),
                                        thread_ids: &thread_ids,
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
                                [view.evidence.clone()],
                                "Codex cumulative usage decreased and opened a new counter epoch",
                            ));
                        }
                        let Some(last) = last else { continue };
                        let estimated = last.input.unwrap_or(0) == 0
                            && last.output.unwrap_or(0) == 0
                            && last.total.unwrap_or(0) > 0;
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
                                [view.evidence.clone()],
                                "Codex emitted an estimate with zero input and output tokens",
                            ));
                            continue;
                        }
                        if step.event != CounterEvent::Repeated {
                            observations.push(counter_observation(
                                &view,
                                last,
                                total,
                                CounterObservation {
                                    role: ObservationRole::Original,
                                    owner: file_thread_text,
                                    thread_ids: &thread_ids,
                                    context: current_turn.and_then(|turn| turns.get(&turn)),
                                    delta: Some(step.delta),
                                },
                            )?);
                        }
                    }
                }
            }
        }
    }

    let mut ledger = reconcile(
        ReconcileInput {
            threads: threads.into_values().collect(),
            relationships,
            requests: observations,
            limit_observations,
            diagnostics,
            ..ReconcileInput::default()
        },
        &LatestRevision,
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

fn legacy_copied_evidence(
    source: &ParsedSource,
    source_id: &AnalyticalId,
    known_turns: &KnownTurns,
) -> CopiedEvidence {
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
            evidence.push(record.evidence(source_id));
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
    owner: &'a str,
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
    let mut observation = RequestObservation::new(record.evidence.clone());
    observation.role = counter.role;
    observation.owner = counter
        .thread_ids
        .get(counter.owner)
        .cloned()
        .map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    if let Some(thread_id) = counter.thread_ids.get(counter.owner) {
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
    owner: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    previous: &mut BTreeMap<LimitStream, Arc<RateLimits>>,
    observations: &mut Vec<ProviderLimitObservation>,
) {
    let owner_thread = thread_ids.get(owner).cloned();
    for window in &limits.windows {
        let stream = (owner_thread.clone(), limits.limit_name.clone(), *window);
        let repeated = previous.get(&stream).is_some_and(|last| last.native == limits.native);
        previous.insert(stream, Arc::clone(limits));
        if repeated {
            continue;
        }
        observations.push(ProviderLimitObservation {
            limit_name: limits.limit_name.clone(),
            window: Some((*window).to_owned()),
            observed_at: record
                .timestamp
                .map_or(Basis::Unknown, |time| Basis::Observed(time.get())),
            owner_thread: owner_thread.clone(),
            owner_request: None,
            native: limits.native.as_str().into(),
            evidence: record.evidence.clone(),
        });
    }
}

fn source_diagnostics(manifest: &SnapshotManifest) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut locations: BTreeMap<&str, Vec<&crate::sources::manifest::ManifestEntry>> =
        BTreeMap::new();
    for entry in &manifest.entries {
        locations.entry(&entry.locator).or_default().push(entry);
        if let Some(evidence) = &entry.first_malformed {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::MalformedLine,
                None,
                [evidence.clone()],
                "a complete Codex rollout line is malformed",
            ));
        }
        if let (Some(source), Some(tail)) = (&entry.source, entry.cutoff.pending_tail) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::PendingTail,
                None,
                [EvidenceRef {
                    source: source.id.clone(),
                    offset: tail.offset,
                    length: tail.length,
                }],
                "the incomplete final Codex rollout line is pending",
            ));
        }
    }
    for entries in locations.values().filter(|entries| entries.len() > 1) {
        let evidence = entries.iter().filter_map(|entry| {
            entry.source.as_ref().map(|source| EvidenceRef {
                source: source.id.clone(),
                offset: 0,
                length: 0,
            })
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

fn direct_observation(
    record: &RecordView,
    payload: &UsagePayload<'_>,
    file_thread: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    turns: &Turns,
) -> Result<RequestObservation, AdapterError> {
    usage_observation(record, payload, ObservationRole::Original, file_thread, thread_ids, turns)
}

fn usage_observation(
    record: &RecordView,
    payload: &UsagePayload<'_>,
    default_role: ObservationRole,
    file_thread: &str,
    thread_ids: &BTreeMap<String, AnalyticalId>,
    turns: &Turns,
) -> Result<RequestObservation, AdapterError> {
    let owner = payload.thread_id.unwrap_or(file_thread);
    let mut observation = RequestObservation::new(record.evidence.clone());
    observation.role = if default_role == ObservationRole::Copy || owner != file_thread {
        ObservationRole::Copy
    } else {
        ObservationRole::Original
    };
    observation.owner =
        thread_ids.get(owner).cloned().map_or(OwnerEvidence::None, OwnerEvidence::Proven);
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
    if let Some(context) = payload.root_turn_id.and_then(|turn| turns.get(&turn)) {
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
    let stem =
        name.strip_suffix(".jsonl.zst").or_else(|| name.strip_suffix(".jsonl")).unwrap_or(&name);
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
        CodexUsage, CountReader, Interner, ParsedRecord, RateLimits, RecordKind, SourceDecoder,
        UsageMask, ingest_root, pack_usage,
    };
    use crate::ledger::diagnostics::DiagnosticCode;
    use crate::ledger::identity::AnalyticalId;
    use crate::sources::evidence::EvidenceRef;
    use crate::sources::reader::{RawRecord, RecordDisposition};

    /// Decodes one line as the next line of `decoder`'s rollout.
    fn decode(decoder: &mut SourceDecoder, line: &str) -> RecordDisposition {
        let source = AnalyticalId::parse("src-v1-00000000000000000000000000").unwrap();
        let evidence =
            EvidenceRef { source, offset: 0, length: u64::try_from(line.len()).unwrap() };
        decoder.decode(&RawRecord { evidence: &evidence, bytes: line.as_bytes() })
    }

    #[test]
    fn admission_does_not_charge_metadata_without_usage() {
        let budget = crate::sources::admission::Admission::new(0);
        let source = AnalyticalId::parse("src-v1-00000000000000000000000000").unwrap();
        let evidence = EvidenceRef { source, offset: 0, length: 1 };
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
            let budget = crate::sources::admission::Admission::new(1);
            let source = AnalyticalId::parse("src-v1-00000000000000000000000000").unwrap();
            let evidence = EvidenceRef { source, offset: 0, length: 1 };
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
                        RecordKind::TokenCount { usage: mask, limits: None }
                    } else {
                        RecordKind::UsageRecord(super::UsageRecord {
                            thread_id: None,
                            response_id: None,
                            root_turn_id: None,
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
