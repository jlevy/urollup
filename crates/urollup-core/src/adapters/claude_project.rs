//! Claude Code project transcript adapter (`claude-project`).
//!
//! Decoding keeps memory proportional to usage records, not to log bytes:
//!
//! - Each line is first read by `LineHead`, which builds no JSON document. A usage-bearing
//!   line is read again as a typed usage body, which keeps accounting fields and never
//!   a parent `serde_json::Value`.
//! - A decoded `ParsedRecord` is a compact row: repeated strings are interned per
//!   ingestion, native IDs that only join records are 128-bit digests, and rare fields
//!   are boxed.
//! - Owner maps run over the decoded records; each source's records are observed and
//!   dropped before the next source's payload is kept beside the observation vector.

mod line;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use jiff::Timestamp;
use serde_json::Value;

use self::line::{LineHead, LineType, RecordFields, UsageBody};
use super::{AdapterError, Ingested};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{
    Basis, Confidence, ModelBasis, ModelName, ModelUsage, ProviderLimitObservation, Relationship,
    RelationshipKind, SourceArtifact, SourceCapability, Thread,
};
use crate::ledger::identity::{AnalyticalId, IdPrefix, KeyComponent, StoredIdentity, sha256_128};
use crate::ledger::names::Name;
use crate::ledger::reconcile::{
    NativeSequence, ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation,
    RevisionChoice, RevisionSelector, reconcile,
};
use crate::ledger::scope::{
    ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec, ScopedKey,
};
use crate::ledger::tokens::TokenMeasures;
use crate::selection::{Agent, agent_thread_identity};
use crate::sources::admission::Admission;
use crate::sources::decode::{parse_timestamp, text};
use crate::sources::evidence::{EvidenceRef, SourceTable};
use crate::sources::manifest::{Fingerprint, ManifestEntry, SkippedLink, SnapshotManifest};
use crate::sources::parallel::{default_workers, source_weight, try_read_in_parallel};
use crate::sources::reader::{RawRecord, ReadOptions, RecordDisposition, SourceSpec, read_source};
use crate::sources::roots::{DiscoveredSource, Discovery, discover};

const DIALECT: &str = "claude-project";
const AGENT_NAMESPACE: &str = "claude";
const PROVIDER_NAMESPACE: &str = "anthropic";
// Sidecars contain only launch metadata, so this hard ceiling bounds the allocation for a
// damaged or unexpected sidecar.
const MAX_SUBAGENT_META_BYTES: u64 = 1024 * 1024;

const REQUEST_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("provider", ComponentRole::Namespace),
    ComponentSlot::required("native_id", ComponentRole::NativeId),
];

const RESPONSE_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "provider-response",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Provider,
    slots: REQUEST_SLOTS,
};

const REQUEST_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "provider-request",
    precedence: 1,
    basis: IdentityBasis::Native,
    scope: IdScope::Provider,
    slots: REQUEST_SLOTS,
};

const AMBIGUOUS_RESPONSE_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "agent-response",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Agent,
    slots: REQUEST_SLOTS,
};

const INLINE_THREAD_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("agent", ComponentRole::Namespace),
    ComponentSlot::required("first_record_digest", ComponentRole::Digest),
];

const INLINE_THREAD_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Thread,
    kind: "inline-sidechain-digest",
    precedence: 1,
    basis: IdentityBasis::Fallback,
    scope: IdScope::Agent,
    slots: INLINE_THREAD_SLOTS,
};

/// The largest a decoded record may be.
///
/// One record is held for every usage line of a whole history until its observation is
/// built, so this size bounds that phase: the 381,000 records of a 2.8 GB corpus take
/// 73 MiB inline. The layout is exactly 200 bytes: 16 of evidence position, a 12-byte
/// thread, four 4-byte symbols, a 32-byte message ID with its digest, a 16-byte request
/// ID, a 16-byte timestamp, 64 bytes of counts with a 1-byte presence mask, a 17-byte
/// optional uuid digest, two flags and an 8-byte pointer to rare fields.
const _: () =
    assert!(size_of::<ParsedRecord>() <= 200, "a decoded Claude record outgrew its size budget");

/// A 128-bit SHA-256 digest of a native ID that only joins records, never appears in
/// output, and so need not be kept as text.
type Digest = [u8; 16];

fn digest(text: &str) -> Digest {
    sha256_128(text.as_bytes())
}

/// An interned string: its position in a string table, plus one.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Sym(NonZeroU32);

impl Sym {
    fn index(self) -> usize {
        usize::try_from(self.0.get()).map_or(usize::MAX, |position| position.saturating_sub(1))
    }
}

/// Strings interned per source while it decodes, then merged into one table per ingestion
/// in discovery order, so symbols never depend on the worker count.
///
/// Symbols from one table compare equal exactly when their strings do.
#[derive(Debug, Default)]
struct Strings {
    symbols: HashMap<Arc<str>, Sym>,
    strings: Vec<Arc<str>>,
}

impl Strings {
    fn intern(&mut self, text: &str) -> Sym {
        match self.symbols.get(text) {
            Some(sym) => *sym,
            None => self.insert(Arc::from(text)),
        }
    }

    /// Adds a string that is not in the table.
    fn insert(&mut self, text: Arc<str>) -> Sym {
        let sym = u32::try_from(self.strings.len().saturating_add(1))
            .ok()
            .and_then(NonZeroU32::new)
            .map(Sym)
            .expect("an ingestion has fewer than 2^32 distinct strings");
        self.strings.push(Arc::clone(&text));
        self.symbols.insert(text, sym);
        sym
    }

    fn resolve(&self, sym: Sym) -> &str {
        &self.strings[sym.index()]
    }

    /// Drops the intern map after a source has finished decoding. [`Self::absorb`] only
    /// needs the string vector, and the map would otherwise wait on the join with every
    /// other source.
    fn freeze(&mut self) {
        self.symbols = HashMap::new();
        self.strings.shrink_to_fit();
    }

    /// Merges another table into this one, returning where each of its symbols went.
    fn absorb(&mut self, other: Self) -> Remap {
        Remap(
            other
                .strings
                .into_iter()
                .map(|text| match self.symbols.get(&text) {
                    Some(sym) => *sym,
                    None => self.insert(text),
                })
                .collect(),
        )
    }
}

/// Where each symbol of a merged table went.
struct Remap(Vec<Sym>);

impl Remap {
    fn sym(&self, sym: Sym) -> Sym {
        self.0[sym.index()]
    }

    fn thread(&self, thread: NativeThread) -> NativeThread {
        NativeThread {
            session: self.sym(thread.session),
            agent: thread.agent.map(|agent| self.sym(agent)),
            inline_digest: thread.inline_digest.map(|digest| self.sym(digest)),
        }
    }
}

/// A timestamp as seconds and biased nanoseconds, whose niche keeps an optional one at
/// 16 bytes instead of 24.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RecordTime {
    second: i64,
    /// Sub-second nanoseconds plus one billion, which is never zero.
    biased_nanosecond: NonZeroU32,
}

impl RecordTime {
    const BIAS: i32 = 1_000_000_000;

    fn new(timestamp: Timestamp) -> Self {
        let biased = timestamp.subsec_nanosecond().saturating_add(Self::BIAS);
        Self {
            second: timestamp.as_second(),
            biased_nanosecond: u32::try_from(biased)
                .ok()
                .and_then(NonZeroU32::new)
                .expect("sub-second nanoseconds lie strictly within one second"),
        }
    }

    fn get(self) -> Timestamp {
        let nanosecond = i32::try_from(self.biased_nanosecond.get())
            .map_or(0, |biased| biased.saturating_sub(Self::BIAS));
        Timestamp::new(self.second, nanosecond).expect("a decomposed timestamp recomposes")
    }
}

/// A number a request record reports; each may be absent.
#[derive(Clone, Copy, Debug)]
enum Count {
    Input,
    CacheRead,
    CacheWrite,
    CacheWrite5m,
    CacheWrite1h,
    Output,
    Reasoning,
    /// `apiBlockIndex`, the position of a block record within its response.
    BlockIndex,
}

const COUNTS: usize = 8;

/// One decoded request-bearing record.
///
/// Its evidence source is the source ID of the chunk that holds it.
struct ParsedRecord {
    offset: u64,
    length: u64,
    thread: NativeThread,
    session: Option<Sym>,
    project: Option<Sym>,
    model: Option<Sym>,
    effort: Option<Sym>,
    forced_copy: bool,
    request_record: bool,
    /// The digest of `uuid`, which only joins replays to originals.
    uuid: Option<Digest>,
    message: Option<MessageId>,
    request_id: Option<Sym>,
    timestamp: Option<RecordTime>,
    /// Values by [`Count`]; a value is present when its bit in `counted` is set.
    counts: [u64; COUNTS],
    counted: u8,
    extras: Option<Box<RecordExtras>>,
}

/// A provider message ID and its digest, which keys ownership and ambiguity.
#[derive(Clone, Copy)]
struct MessageId {
    digest: Digest,
    text: Sym,
}

/// The fields few records have, boxed so that other records do not pay for them.
struct RecordExtras {
    advisors: Vec<AdvisorUsage>,
    quota: Option<QuotaLimits>,
    limit_text: Option<Box<str>>,
}

/// A record's `quotaLimits` object, already in the form a limit observation keeps.
struct QuotaLimits {
    limit_name: Option<Box<str>>,
    native: Box<str>,
}

#[derive(Clone, Debug)]
struct AdvisorUsage {
    model: Option<String>,
    input: Option<u64>,
    cache_read: Option<u64>,
    cache_write: Option<u64>,
    output: Option<u64>,
    reasoning: Option<u64>,
}

impl ParsedRecord {
    fn count(&self, count: Count) -> Option<u64> {
        let bit = 1_u8 << (count as u8);
        (self.counted & bit != 0).then(|| self.counts[count as usize])
    }

    fn evidence(&self, source: u32) -> EvidenceRef {
        EvidenceRef::new(source, self.offset, self.length)
    }

    fn advisors(&self) -> &[AdvisorUsage] {
        self.extras.as_ref().map_or(&[], |extras| &extras.advisors)
    }

    /// The session the record names, or its file's session when it names none.
    fn recorded_session(&self) -> Sym {
        self.session.unwrap_or(self.thread.session)
    }

    /// Whether a main-session record names another session, as a resumed session's
    /// replay does.
    fn is_foreign_session(&self) -> bool {
        !self.thread.is_child() && self.recorded_session() != self.thread.session
    }

    /// Whether the record may own its message and uuid: a request record that is neither a
    /// nested copy nor a replay recorded under another session.
    fn is_original_eligible(&self) -> bool {
        self.request_record && !self.forced_copy && !self.is_foreign_session()
    }

    /// The thread that recorded the request natively: the inline sidechain itself, or the
    /// recorded session with the file's agent.
    fn native_owner(&self) -> NativeThread {
        if self.thread.inline_digest.is_some() {
            self.thread
        } else {
            NativeThread {
                session: self.recorded_session(),
                agent: self.thread.agent,
                inline_digest: None,
            }
        }
    }

    fn is_replayed(&self, owners: &Owners) -> bool {
        if self.forced_copy || self.is_foreign_session() {
            return true;
        }
        let message_replayed = self
            .message
            .as_ref()
            .and_then(|message| owners.messages.get(&message.digest))
            .is_some_and(|owner| self.thread.is_child() && *owner != self.thread);
        let uuid_replayed = self
            .uuid
            .and_then(|uuid| owners.uuids.get(&uuid))
            .is_some_and(|owner| *owner != self.thread);
        message_replayed || uuid_replayed
    }

    fn remap(&mut self, remap: &Remap) {
        self.thread = remap.thread(self.thread);
        for sym in [
            &mut self.session,
            &mut self.project,
            &mut self.model,
            &mut self.effort,
            &mut self.request_id,
        ] {
            *sym = sym.map(|sym| remap.sym(sym));
        }
        if let Some(message) = &mut self.message {
            message.text = remap.sym(message.text);
        }
    }
}

struct SourceFacts {
    thread: NativeThread,
    /// The snapshot `src-` ID, set after the reader fingerprints the first record.
    source_id: Option<AnalyticalId>,
    evidence: Option<EvidenceRef>,
    version: Option<String>,
    project: Option<Sym>,
    last_main_evidence: Option<EvidenceRef>,
    active_inline: Option<NativeThread>,
    inline_threads: Vec<(NativeThread, EvidenceRef, Option<EvidenceRef>)>,
}

impl SourceFacts {
    fn remap(&mut self, remap: &Remap) {
        self.thread = remap.thread(self.thread);
        self.project = self.project.map(|project| remap.sym(project));
        self.active_inline = self.active_inline.map(|inline| remap.thread(inline));
        for (child, _, _) in &mut self.inline_threads {
            *child = remap.thread(*child);
        }
    }
}

struct SubagentMeta {
    child: NativeThread,
    tool_use: Option<Digest>,
    agent_type: Option<String>,
    source_id: AnalyticalId,
    evidence: EvidenceRef,
}

/// A thread by its native strings, interned.
///
/// Symbols stand in for strings in equality and hashing; ordering compares the strings,
/// through [`NativeThread::text_order`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct NativeThread {
    session: Sym,
    agent: Option<Sym>,
    inline_digest: Option<Sym>,
}

impl NativeThread {
    fn main(session: Sym) -> Self {
        Self { session, agent: None, inline_digest: None }
    }

    fn inline(session: Sym, first_record: &[u8], strings: &mut Strings) -> Self {
        Self {
            session,
            agent: None,
            inline_digest: Some(strings.intern(&Fingerprint::of(first_record).to_base32())),
        }
    }

    fn is_child(self) -> bool {
        self.agent.is_some() || self.inline_digest.is_some()
    }

    /// The order of the threads' strings: session, then agent, then inline digest, with
    /// an absent part first.
    fn text_order(self, other: Self, strings: &Strings) -> Ordering {
        let parts = |thread: Self| {
            (
                strings.resolve(thread.session),
                thread.agent.map(|agent| strings.resolve(agent)),
                thread.inline_digest.map(|digest| strings.resolve(digest)),
            )
        };
        parts(self).cmp(&parts(other))
    }

    fn native_id(self, strings: &Strings) -> String {
        let session = strings.resolve(self.session);
        match self.agent {
            Some(agent) => format!("{session}/{}", strings.resolve(agent)),
            None => session.to_owned(),
        }
    }

    fn identity(self, strings: &Strings) -> Result<StoredIdentity, AdapterError> {
        let key = match self.inline_digest {
            Some(digest) => {
                INLINE_THREAD_KEY
                    .key(vec![
                        KeyComponent::text(AGENT_NAMESPACE),
                        KeyComponent::text(strings.resolve(digest)),
                    ])?
                    .key
            }
            None => return Ok(agent_thread_identity(Agent::Claude, &self.native_id(strings))?),
        };
        Ok(StoredIdentity::derive(key)?)
    }

    fn basis(self) -> IdentityBasis {
        if self.inline_digest.is_some() { IdentityBasis::Fallback } else { IdentityBasis::Native }
    }
}

/// Reads one Claude Code configuration root, including `projects/` and subagents.
pub fn ingest_root(root: &Path) -> Result<Ingested, AdapterError> {
    ingest_roots(&[root.to_owned()], true)
}

/// Reads Claude Code transcript roots selected by discovery.
///
/// Missing variable- or flag-selected roots are errors; missing conventional defaults
/// are skipped.
pub fn ingest_roots(roots: &[PathBuf], missing_is_error: bool) -> Result<Ingested, AdapterError> {
    let discovery = discover(roots);
    ingest_discovery(discovery, missing_is_error)
}

/// Reads a previously discovered set of Claude Code transcripts.
///
/// Callers that need exact session selection can filter `discovery.sources` before
/// invoking this function, avoiding a second walk and full ingestion of unrelated
/// transcripts while preserving paired plain/compressed representations. Sources decode
/// on the [default worker bound](default_workers).
pub fn ingest_discovery(
    discovery: Discovery,
    missing_is_error: bool,
) -> Result<Ingested, AdapterError> {
    ingest_discovery_with_workers(discovery, missing_is_error, default_workers())
}

/// Reads discovered Claude Code transcripts on at most `workers` threads.
///
/// Each source decodes independently, and the results merge in discovery order before
/// normalization, so the result is the same for every worker count. A failure returns
/// the error of the first failing source in discovery order, as a sequential read does.
/// Exhausting the shared admission budget instead aborts the invocation with a capacity
/// error; it takes precedence over errors from sources interrupted by that refusal.
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
    let (corpus, manifest) = Corpus::merge(decoded, discovery.skipped_links);
    normalize(corpus, manifest)
}

/// Everything one transcript contributes before normalization.
struct DecodedSource {
    entry: ManifestEntry,
    facts: SourceFacts,
    subagent_meta: Option<SubagentMeta>,
    records: Vec<ParsedRecord>,
    /// Tool-use IDs in record order, with the thread of the record that issued each.
    tool_uses: Vec<(Digest, NativeThread)>,
    /// The table that this source's symbols index.
    strings: Strings,
}

/// Reads one transcript and its subagent sidecar, independently of every other source.
fn decode_source(
    source: &DiscoveredSource,
    admission: &Admission,
) -> Result<DecodedSource, AdapterError> {
    let mut decoder = SourceDecoder::new(&source.locator);
    let spec = SourceSpec {
        environment: "local",
        dialect: DIALECT,
        locator: &source.locator,
        stable_locator: false,
    };
    let path = source
        .files
        .primary()
        .map_or_else(|| source.root.join(&source.locator), |(path, _)| path.to_owned());
    let entry = read_source(&spec, &source.files, &ReadOptions::default(), |raw| {
        decoder.decode_with_admission(raw, Some(admission))
    })
    .map_err(|source| AdapterError::Read { path: path.clone(), source })?;
    let SourceDecoder { thread, mut strings, mut facts, mut records, mut tool_uses } = decoder;
    if let Some(identity) = &entry.source {
        facts.source_id = Some(identity.id.clone());
    }
    // Decoding grows the vectors by doubling; release the unused tails and the intern
    // map before the source waits for the other workers.
    records.shrink_to_fit();
    tool_uses.shrink_to_fit();
    strings.freeze();
    let mut subagent_meta = None;
    if thread.agent.is_some() {
        if let (Some(identity), Some(meta)) = (entry.source.as_ref(), read_subagent_meta(&path)?) {
            subagent_meta = Some(SubagentMeta {
                child: thread,
                tool_use: text(&meta, &["toolUseId"]).map(digest),
                agent_type: text(&meta, &["agentType"]).map(str::to_owned),
                source_id: identity.id.clone(),
                evidence: EvidenceRef::new(0, 0, 0),
            });
        }
    }
    Ok(DecodedSource { entry, facts, subagent_meta, records, tool_uses, strings })
}

fn read_subagent_meta(transcript: &Path) -> Result<Option<Value>, AdapterError> {
    read_subagent_meta_with_limit(transcript, MAX_SUBAGENT_META_BYTES)
}

fn read_subagent_meta_with_limit(
    transcript: &Path,
    max_sidecar_bytes: u64,
) -> Result<Option<Value>, AdapterError> {
    let transcript = if transcript.extension() == Some(std::ffi::OsStr::new("zst")) {
        transcript.with_extension("")
    } else {
        transcript.to_owned()
    };
    let path = transcript.with_extension("meta.json");
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(AdapterError::MetadataRead { path, source }),
    };
    let mut bytes = Vec::new();
    file.take(max_sidecar_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| AdapterError::MetadataRead { path: path.clone(), source })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_sidecar_bytes {
        return Err(AdapterError::MetadataTooLarge { path, maximum: max_sidecar_bytes });
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| AdapterError::MetadataParse { path, source })
}

/// The state of one transcript while its lines decode.
struct SourceDecoder {
    thread: NativeThread,
    strings: Strings,
    facts: SourceFacts,
    records: Vec<ParsedRecord>,
    tool_uses: Vec<(Digest, NativeThread)>,
}

impl SourceDecoder {
    fn new(locator: &str) -> Self {
        let mut strings = Strings::default();
        let thread = thread_from_path(locator, &mut strings);
        Self {
            thread,
            strings,
            facts: SourceFacts {
                thread,
                source_id: None,
                evidence: None,
                version: None,
                project: None,
                last_main_evidence: None,
                active_inline: None,
                inline_threads: Vec::new(),
            },
            records: Vec::new(),
            tool_uses: Vec::new(),
        }
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
        let facts = &mut self.facts;
        if facts.evidence.is_none() {
            facts.evidence = Some(*raw.evidence);
        }
        let Ok(head) = LineHead::read(raw.bytes, facts.version.is_none(), facts.project.is_none())
        else {
            return RecordDisposition::Malformed;
        };
        if facts.version.is_none() {
            facts.version = head.version.as_deref().map(str::to_owned);
        }
        if facts.project.is_none() {
            facts.project =
                head.cwd.as_deref().and_then(project_name).map(|name| self.strings.intern(&name));
        }
        let record_thread = if self.thread.is_child() || !head.sidechain {
            facts.active_inline = None;
            facts.last_main_evidence = Some(*raw.evidence);
            self.thread
        } else if let Some(inline) = facts.active_inline {
            inline
        } else {
            // The first record of a run of sidechain records starts an inline thread.
            let child = NativeThread::inline(self.thread.session, raw.bytes, &mut self.strings);
            facts.inline_threads.push((child, *raw.evidence, facts.last_main_evidence));
            facts.active_inline = Some(child);
            child
        };
        if !head.bears_usage() {
            return RecordDisposition::Skipped;
        }
        // The head proved the line is a valid document that bears usage. The body pass
        // keeps accounting fields and never builds a parent document.
        let Ok(body) = UsageBody::read(raw.bytes) else {
            return RecordDisposition::Malformed;
        };
        let (fields, session, forced_copy, request_record) = match head.line_type {
            LineType::Assistant => {
                let synthetic_error = body.top.message.model.as_deref() == Some("<synthetic>")
                    && body.top.request_id.is_none();
                (&body.top, body.top.session.as_deref(), false, !synthetic_error)
            }
            LineType::Progress => {
                let Some(nested) = &body.nested else {
                    return RecordDisposition::Skipped;
                };
                // A nested record takes the session of the progress record that carries it.
                let session = body.top.session.as_deref().or(nested.session.as_deref());
                (nested, session, true, true)
            }
            LineType::Other => return RecordDisposition::Skipped,
        };
        if request_record && admission.is_some_and(|budget| !budget.reserve()) {
            return RecordDisposition::Stop;
        }
        let record =
            self.record(raw.evidence, record_thread, fields, session, forced_copy, request_record);
        self.records.push(record);
        RecordDisposition::Decoded
    }

    /// The accounting fields of a request-bearing record.
    fn record(
        &mut self,
        evidence: &EvidenceRef,
        thread: NativeThread,
        fields: &RecordFields<'_>,
        session: Option<&str>,
        forced_copy: bool,
        request_record: bool,
    ) -> ParsedRecord {
        let strings = &mut self.strings;
        let mut counts = [0; COUNTS];
        let mut counted = 0_u8;
        let mut set = |count: Count, value: Option<u64>| {
            if let Some(number) = value {
                counts[count as usize] = number;
                counted |= 1 << (count as u8);
            }
        };
        set(Count::Input, fields.message.usage.input);
        set(Count::CacheRead, fields.message.usage.cache_read);
        set(Count::CacheWrite, fields.message.usage.cache_write);
        set(Count::CacheWrite5m, fields.message.usage.cache_write_5m);
        set(Count::CacheWrite1h, fields.message.usage.cache_write_1h);
        set(Count::Output, fields.message.usage.output);
        set(Count::Reasoning, fields.message.usage.reasoning);
        set(Count::BlockIndex, fields.api_block_index);
        for tool_use_id in &fields.message.tool_use_ids {
            self.tool_uses.push((digest(tool_use_id), thread));
        }
        ParsedRecord {
            offset: evidence.offset,
            length: u64::from(evidence.length),
            thread,
            session: session.map(|session| strings.intern(session)),
            project: fields
                .cwd
                .as_deref()
                .and_then(project_name)
                .map(|project| strings.intern(&project)),
            model: fields.message.model.as_deref().map(|model| strings.intern(model)),
            effort: fields.effort.as_deref().map(|effort| strings.intern(effort)),
            forced_copy,
            request_record,
            uuid: fields.uuid.as_deref().map(digest),
            message: fields
                .message
                .id
                .as_deref()
                .map(|id| MessageId { digest: digest(id), text: strings.intern(id) }),
            request_id: fields.request_id.as_deref().map(|id| strings.intern(id)),
            timestamp: fields
                .timestamp
                .as_deref()
                .and_then(|timestamp| parse_timestamp(timestamp).ok())
                .map(RecordTime::new),
            counts,
            counted,
            extras: record_extras(fields),
        }
    }
}

/// The project name a working directory implies: its last component.
fn project_name(cwd: &str) -> Option<Cow<'_, str>> {
    Path::new(cwd).file_name().map(|name| name.to_string_lossy())
}

fn record_extras(fields: &RecordFields<'_>) -> Option<Box<RecordExtras>> {
    let extras = RecordExtras {
        advisors: advisor_usage(fields),
        quota: quota_limits(fields),
        limit_text: usage_limit_text(fields).map(Box::from),
    };
    (!extras.advisors.is_empty() || extras.quota.is_some() || extras.limit_text.is_some())
        .then(|| Box::new(extras))
}

fn advisor_usage(fields: &RecordFields<'_>) -> Vec<AdvisorUsage> {
    fields
        .message
        .advisors
        .iter()
        .map(|advisor| AdvisorUsage {
            model: advisor.model.as_deref().map(str::to_owned),
            input: advisor.input,
            cache_read: advisor.cache_read,
            cache_write: advisor.cache_write,
            output: advisor.output,
            reasoning: advisor.reasoning,
        })
        .collect()
}

fn quota_limits(fields: &RecordFields<'_>) -> Option<QuotaLimits> {
    let quota = fields.quota.as_ref()?;
    let sorted: BTreeMap<&String, &Value> = quota.iter().collect();
    Some(QuotaLimits {
        limit_name: quota.get("rateLimitType").and_then(Value::as_str).map(Box::from),
        native: serde_json::to_string(&sorted).unwrap_or_default().into_boxed_str(),
    })
}

fn usage_limit_text<'a>(fields: &'a RecordFields<'_>) -> Option<&'a str> {
    fields
        .is_api_error
        .then_some(fields.message.first_text.as_deref())
        .flatten()
        .filter(|message| message.starts_with("Claude AI usage limit reached"))
}

/// One source's records and the source ID they share: a reader assigns one ID to every
/// record of a scan.
struct RecordChunk {
    source: AnalyticalId,
    records: Vec<ParsedRecord>,
}

/// Every decoded source merged in discovery order under one string table, with records
/// left in their per-source chunks so they are never copied into one vector.
struct Corpus {
    strings: Strings,
    chunks: Vec<RecordChunk>,
    facts: Vec<SourceFacts>,
    subagent_meta: Vec<SubagentMeta>,
    /// The thread of the first record, in discovery and record order, that issued each
    /// tool use a subagent sidecar names.
    tool_owners: HashMap<Digest, NativeThread>,
}

impl Corpus {
    fn merge(
        decoded: Vec<DecodedSource>,
        skipped_links: Vec<SkippedLink>,
    ) -> (Self, SnapshotManifest) {
        let spawning_tool_uses: HashSet<Digest> =
            decoded.iter().filter_map(|source| source.subagent_meta.as_ref()?.tool_use).collect();
        let mut corpus = Self {
            strings: Strings::default(),
            chunks: Vec::with_capacity(decoded.len()),
            facts: Vec::with_capacity(decoded.len()),
            subagent_meta: Vec::new(),
            tool_owners: HashMap::new(),
        };
        let mut manifest =
            SnapshotManifest { entries: Vec::with_capacity(decoded.len()), skipped_links };
        for source in decoded {
            let DecodedSource { entry, mut facts, subagent_meta, mut records, tool_uses, strings } =
                source;
            let remap = corpus.strings.absorb(strings);
            facts.remap(&remap);
            for record in &mut records {
                record.remap(&remap);
            }
            if let (Some(id), false) = (&facts.source_id, records.is_empty()) {
                corpus.chunks.push(RecordChunk { source: id.clone(), records });
            }
            for (tool_use, thread) in tool_uses {
                if spawning_tool_uses.contains(&tool_use) {
                    corpus.tool_owners.entry(tool_use).or_insert_with(|| remap.thread(thread));
                }
            }
            corpus.subagent_meta.extend(
                subagent_meta.map(|meta| SubagentMeta { child: remap.thread(meta.child), ..meta }),
            );
            manifest.entries.push(entry);
            corpus.facts.push(facts);
        }
        (corpus, manifest)
    }

    fn records(&self) -> impl Iterator<Item = (&AnalyticalId, &ParsedRecord)> {
        self.chunks
            .iter()
            .flat_map(|chunk| chunk.records.iter().map(move |record| (&chunk.source, record)))
    }
}

/// Which thread first recorded each message and uuid, by digest.
struct Owners {
    messages: HashMap<Digest, NativeThread>,
    uuids: HashMap<Digest, NativeThread>,
}

impl Owners {
    /// Owners come only from records that can be originals, in an order that does not
    /// depend on file names: a resumed session's replay never claims the original's IDs.
    fn new(corpus: &Corpus) -> Self {
        let mut candidates: Vec<(&AnalyticalId, &ParsedRecord)> =
            corpus.records().filter(|(_, row)| row.is_original_eligible()).collect();
        candidates.sort_by(compare_owner_precedence);
        let mut owners = Self { messages: HashMap::new(), uuids: HashMap::new() };
        for (_, row) in candidates {
            if let Some(message) = &row.message {
                owners.messages.entry(message.digest).or_insert(row.thread);
            }
            if let Some(uuid) = row.uuid {
                owners.uuids.entry(uuid).or_insert(row.thread);
            }
        }
        owners
    }
}

/// The order in which eligible records claim message and uuid ownership: main-session
/// records before subagent records, then the earliest timestamp, then evidence position.
fn compare_owner_precedence(
    (left_source, left): &(&AnalyticalId, &ParsedRecord),
    (right_source, right): &(&AnalyticalId, &ParsedRecord),
) -> Ordering {
    left.thread
        .is_child()
        .cmp(&right.thread.is_child())
        .then_with(|| left.timestamp.is_none().cmp(&right.timestamp.is_none()))
        .then_with(|| {
            left.timestamp.map(RecordTime::get).cmp(&right.timestamp.map(RecordTime::get))
        })
        .then_with(|| left_source.cmp(right_source))
        .then_with(|| left.offset.cmp(&right.offset))
        .then_with(|| left.length.cmp(&right.length))
}

/// Digests of the message IDs that non-replayed request records report with more than one
/// model.
fn ambiguous_messages(corpus: &Corpus, owners: &Owners) -> HashSet<Digest> {
    // The first model seen for each message, or `None` once a second one appears.
    let mut models: HashMap<Digest, Option<Sym>> = HashMap::new();
    for (_, row) in corpus.records() {
        if !row.request_record || row.is_replayed(owners) {
            continue;
        }
        if let (Some(message), Some(model)) = (&row.message, row.model) {
            models
                .entry(message.digest)
                .and_modify(|first| {
                    if *first != Some(model) {
                        *first = None;
                    }
                })
                .or_insert(Some(model));
        }
    }
    models.into_iter().filter_map(|(message, first)| first.is_none().then_some(message)).collect()
}

fn normalize(corpus: Corpus, manifest: SnapshotManifest) -> Result<Ingested, AdapterError> {
    let mut source_versions: BTreeMap<AnalyticalId, String> = BTreeMap::new();
    for facts in &corpus.facts {
        if let (Some(id), Some(version)) = (&facts.source_id, &facts.version) {
            source_versions.entry(id.clone()).or_insert_with(|| version.clone());
        }
    }
    // Building the input consumes the corpus and drops every map it needed, so only the
    // input is alive while reconciliation reaches its peak.
    let input = reconcile_input(corpus)?;
    let mut ledger = reconcile(input, &ClaudeBlockSelector)?;
    for diagnostic in &mut ledger.diagnostics {
        diagnostic.code = match diagnostic.code {
            DiagnosticCode::RevisionDisagreement => DiagnosticCode::ClaudeBlockUsageConflict,
            DiagnosticCode::UsageInconsistency => {
                DiagnosticCode::ClaudeCacheCreationBreakdownMismatch
            }
            DiagnosticCode::CopyWithoutOriginal => DiagnosticCode::ClaudeNestedCopyWithoutOriginal,
            DiagnosticCode::ConflictingSharedKey => DiagnosticCode::IdentityKeyConflict,
            code @ (DiagnosticCode::ConflictingReread
            | DiagnosticCode::ConflictingOwners
            | DiagnosticCode::ConflictingAccounts
            | DiagnosticCode::ConflictingModels
            | DiagnosticCode::UnresolvedCandidate
            | DiagnosticCode::CounterReset
            | DiagnosticCode::CounterGap
            | DiagnosticCode::ClaudeBlockUsageConflict
            | DiagnosticCode::ClaudeCacheCreationBreakdownMismatch
            | DiagnosticCode::ClaudeNestedCopyWithoutOriginal
            | DiagnosticCode::IdentityKeyConflict
            | DiagnosticCode::CodexCopiedHistoryInferred
            | DiagnosticCode::CodexCounterEpochReset
            | DiagnosticCode::CodexEstimateCompaction
            | DiagnosticCode::CodexEstimateContextWindowFill
            | DiagnosticCode::CodexRolloutDuplicateLocation
            | DiagnosticCode::MalformedLine
            | DiagnosticCode::PendingTail
            | DiagnosticCode::ThreadOrphan) => code,
        };
    }
    ledger.diagnostics.sort();
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

fn source_index(table: &SourceTable, source: &AnalyticalId) -> u32 {
    table.index_of(source).expect("every cited source was interned")
}

fn stamp_ref(table: &SourceTable, id: &AnalyticalId, evidence: EvidenceRef) -> EvidenceRef {
    evidence.with_source(source_index(table, id))
}

/// Builds the reconciliation input, consuming the corpus chunk by chunk.
fn reconcile_input(mut corpus: Corpus) -> Result<ReconcileInput, AdapterError> {
    let source_table = SourceTable::from_ids(
        corpus
            .chunks
            .iter()
            .map(|chunk| chunk.source.clone())
            .chain(corpus.facts.iter().filter_map(|facts| facts.source_id.clone()))
            .chain(corpus.subagent_meta.iter().map(|meta| meta.source_id.clone())),
    );
    for facts in &mut corpus.facts {
        let Some(id) = facts.source_id.as_ref() else { continue };
        if let Some(evidence) = facts.evidence {
            facts.evidence = Some(stamp_ref(&source_table, id, evidence));
        }
        if let Some(evidence) = facts.last_main_evidence {
            facts.last_main_evidence = Some(stamp_ref(&source_table, id, evidence));
        }
        for (_, evidence, parent) in &mut facts.inline_threads {
            *evidence = stamp_ref(&source_table, id, *evidence);
            if let Some(parent) = parent {
                *parent = stamp_ref(&source_table, id, *parent);
            }
        }
    }
    for meta in &mut corpus.subagent_meta {
        meta.evidence = stamp_ref(&source_table, &meta.source_id, meta.evidence);
    }
    let owners = Owners::new(&corpus);
    let ambiguous = ambiguous_messages(&corpus, &owners);
    let mut diagnostics = ambiguity_diagnostics(&corpus, &ambiguous, &owners, &source_table);
    let (threads, relationships, ids) = thread_graph(&corpus, &source_table)?;
    let Corpus { strings, chunks, facts, subagent_meta, tool_owners } = corpus;
    drop((facts, subagent_meta, tool_owners));

    // Reserve per chunk after earlier record vectors have dropped. A single reserve for
    // every record would allocate the observation table (~224 B each) while the corpus
    // is still fully resident beside the Codex ledger.
    let mut observations = Vec::new();
    let mut limit_observations = Vec::new();
    for RecordChunk { source, records } in chunks {
        observations.reserve(records.iter().filter(|record| record.request_record).count());
        for record in records {
            let evidence = record.evidence(source_index(&source_table, &source));
            append_limits(&record, &evidence, &strings, &ids, &ambiguous, &mut limit_observations)?;
            if !record.request_record {
                continue;
            }
            if let Some((flat, breakdown)) = cache_breakdown_mismatch(&record) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::UsageInconsistency,
                    None,
                    [evidence],
                    format!(
                        "Claude cache creation total {flat} differs from its lifetime breakdown {breakdown}"
                    ),
                ));
            }
            observations.push(observe(&record, evidence, &strings, &owners, &ambiguous, &ids)?);
        }
    }

    Ok(ReconcileInput {
        threads,
        relationships,
        requests: observations,
        limit_observations,
        diagnostics,
        source_table,
        ..ReconcileInput::default()
    })
}

/// One conflicting-key diagnostic per ambiguous message, in message ID order, citing its
/// non-replayed records.
fn ambiguity_diagnostics(
    corpus: &Corpus,
    ambiguous: &HashSet<Digest>,
    owners: &Owners,
    sources: &SourceTable,
) -> Vec<Diagnostic> {
    if ambiguous.is_empty() {
        return Vec::new();
    }
    let mut evidence_by_message: BTreeMap<&str, Vec<EvidenceRef>> = BTreeMap::new();
    for (source, record) in corpus.records() {
        let Some(message) = &record.message else { continue };
        if ambiguous.contains(&message.digest) && !record.is_replayed(owners) {
            evidence_by_message
                .entry(corpus.strings.resolve(message.text))
                .or_default()
                .push(record.evidence(source_index(sources, source)));
        }
    }
    evidence_by_message
        .into_iter()
        .map(|(message_id, evidence)| {
            Diagnostic::new(
                DiagnosticCode::ConflictingSharedKey,
                None,
                evidence,
                format!("Claude message ID {message_id} is reused by conflicting responses"),
            )
        })
        .collect()
}

type ThreadGraph = (Vec<Thread>, Vec<Relationship>, HashMap<NativeThread, AnalyticalId>);

/// Threads, their relationships, and the analytical ID of every native thread.
fn thread_graph(corpus: &Corpus, sources: &SourceTable) -> Result<ThreadGraph, AdapterError> {
    let strings = &corpus.strings;
    let mut native_threads: HashSet<NativeThread> =
        corpus.facts.iter().map(|facts| facts.thread).collect();
    let mut relationships = Vec::new();
    for (source, record) in corpus.records() {
        native_threads.insert(record.thread);
        let recorded_session = NativeThread::main(record.recorded_session());
        native_threads.insert(recorded_session);
        if record.is_foreign_session() {
            relationships.push((
                RelationshipKind::Fork,
                recorded_session,
                record.thread,
                record.evidence(source_index(sources, source)),
            ));
        }
    }
    for facts in &corpus.facts {
        for (child, evidence, parent_evidence) in &facts.inline_threads {
            native_threads.insert(*child);
            if let Some(parent_evidence) = parent_evidence {
                relationships.push((
                    RelationshipKind::InlineSidechain,
                    facts.thread,
                    *child,
                    *parent_evidence,
                ));
            }
            relationships.push((
                RelationshipKind::InlineSidechain,
                facts.thread,
                *child,
                *evidence,
            ));
        }
    }
    let mut native_threads: Vec<NativeThread> = native_threads.into_iter().collect();
    native_threads.sort_by(|left, right| left.text_order(*right, strings));

    let purpose_by_thread: HashMap<NativeThread, &str> = corpus
        .subagent_meta
        .iter()
        .filter_map(|meta| meta.agent_type.as_deref().map(|purpose| (meta.child, purpose)))
        .collect();
    let mut spawned = HashSet::new();
    for meta in &corpus.subagent_meta {
        let parent = meta
            .tool_use
            .and_then(|tool_use| corpus.tool_owners.get(&tool_use))
            .copied()
            .unwrap_or_else(|| NativeThread::main(meta.child.session));
        spawned.insert(meta.child);
        relationships.push((RelationshipKind::Spawn, parent, meta.child, meta.evidence));
    }
    // A subagent without a sidecar is spawned by its session, cited by its first record.
    let unspawned: Vec<NativeThread> = native_threads
        .iter()
        .copied()
        .filter(|thread| thread.agent.is_some() && !spawned.contains(thread))
        .collect();
    if !unspawned.is_empty() {
        let wanted: HashSet<NativeThread> = unspawned.iter().copied().collect();
        let mut first_evidence: HashMap<NativeThread, EvidenceRef> = HashMap::new();
        for (source, record) in corpus.records() {
            if wanted.contains(&record.thread) {
                first_evidence
                    .entry(record.thread)
                    .or_insert_with(|| record.evidence(source_index(sources, source)));
            }
        }
        for child in unspawned {
            if let Some(evidence) = first_evidence.remove(&child) {
                relationships.push((
                    RelationshipKind::Spawn,
                    NativeThread::main(child.session),
                    child,
                    evidence,
                ));
            }
        }
    }

    let mut thread_evidence: HashMap<NativeThread, Vec<EvidenceRef>> = HashMap::new();
    let mut project_by_thread: HashMap<NativeThread, Sym> = HashMap::new();
    for facts in &corpus.facts {
        if let Some(evidence) = &facts.evidence {
            thread_evidence.entry(facts.thread).or_default().push(*evidence);
        }
        if let Some(project) = facts.project {
            project_by_thread.entry(facts.thread).or_insert(project);
            for (inline, _, _) in &facts.inline_threads {
                project_by_thread.entry(*inline).or_insert(project);
            }
        }
        for (inline, evidence, _) in &facts.inline_threads {
            thread_evidence.entry(*inline).or_default().push(*evidence);
        }
    }
    for (source, record) in corpus.records() {
        thread_evidence
            .entry(record.thread)
            .or_default()
            .push(record.evidence(source_index(sources, source)));
        if let Some(project) = record.project {
            project_by_thread.entry(record.thread).or_insert(project);
        }
    }
    for evidence in thread_evidence.values_mut() {
        evidence.sort();
        evidence.dedup();
        evidence.shrink_to_fit();
    }

    let mut ids = HashMap::with_capacity(native_threads.len());
    let mut threads = BTreeMap::new();
    for native in native_threads {
        let identity = native.identity(strings)?;
        ids.insert(native, identity.id.clone());
        let mut native_key = BTreeMap::new();
        if native.inline_digest.is_none() {
            native_key.insert("session_id".to_owned(), strings.resolve(native.session).to_owned());
            if let Some(agent) = native.agent {
                native_key.insert("agent_id".to_owned(), strings.resolve(agent).to_owned());
            }
        }
        threads.insert(
            identity.id.clone(),
            Thread {
                identity,
                basis: native.basis(),
                aliases: Vec::new(),
                native_key,
                source: Basis::Observed(
                    if native.inline_digest.is_some() {
                        "inline-sidechain"
                    } else if native.agent.is_some() {
                        "subagent"
                    } else {
                        "cli"
                    }
                    .to_owned(),
                ),
                initiator: Basis::Unknown,
                purpose: purpose_by_thread
                    .get(&native)
                    .map_or(Basis::Unknown, |purpose| Basis::Observed((*purpose).to_owned())),
                execution_environment: Basis::Observed("local".to_owned()),
                project: project_by_thread.get(&native).map_or(Basis::Unknown, |project| {
                    Basis::Observed(strings.resolve(*project).to_owned())
                }),
                account: Basis::Unknown,
                evidence: thread_evidence.remove(&native).unwrap_or_default(),
            },
        );
    }

    let relationships = relationships
        .into_iter()
        .filter_map(|(kind, from, to, evidence)| {
            Some(Relationship {
                confidence: if kind == RelationshipKind::InlineSidechain {
                    Confidence::Inferred
                } else {
                    Confidence::Proven
                },
                kind,
                from: ids.get(&from)?.clone(),
                to: ids.get(&to)?.clone(),
                evidence: vec![evidence],
            })
        })
        .collect();
    Ok((threads.into_values().collect(), relationships, ids))
}

/// The request observation of one request record.
fn observe(
    record: &ParsedRecord,
    evidence: EvidenceRef,
    strings: &Strings,
    owners: &Owners,
    ambiguous: &HashSet<Digest>,
    ids: &HashMap<NativeThread, AnalyticalId>,
) -> Result<RequestObservation, AdapterError> {
    let mut observation = RequestObservation::new(evidence);
    let recorded_session = record.recorded_session();
    if let Some(message) = &record.message {
        observation
            .keys
            .push(response_key(message, recorded_session, strings, ambiguous)?.derive()?);
    }
    if let Some(request_id) = record.request_id {
        observation.keys.push(
            REQUEST_KEY
                .key(vec![
                    KeyComponent::text(PROVIDER_NAMESPACE),
                    KeyComponent::text(strings.resolve(request_id)),
                ])?
                .derive()?,
        );
    }
    let replay_owner = record
        .message
        .as_ref()
        .and_then(|message| owners.messages.get(&message.digest))
        .filter(|owner| {
            record.forced_copy || (record.thread.is_child() && **owner != record.thread)
        })
        .or_else(|| {
            record
                .uuid
                .and_then(|uuid| owners.uuids.get(&uuid))
                .filter(|owner| **owner != record.thread)
        });
    let owner = replay_owner
        .and_then(|owner| ids.get(owner))
        .or_else(|| ids.get(&record.native_owner()))
        .or_else(|| ids.get(&NativeThread::main(recorded_session)));
    observation.owner = owner.cloned().map_or(OwnerEvidence::None, OwnerEvidence::Proven);
    observation.role =
        if record.forced_copy || replay_owner.is_some() || record.is_foreign_session() {
            ObservationRole::Copy
        } else {
            ObservationRole::Original
        };
    if observation.role == ObservationRole::Original {
        let (usage, model_usage) = claude_usage(record, strings)?;
        observation.usage = Some(usage.into());
        observation.model_usage = model_usage.into();
        observation.sequence = record.count(Count::BlockIndex).and_then(NativeSequence::new);
        observation.model = record.model.map(|model| ModelName {
            name: strings.resolve(model).into(),
            basis: ModelBasis::Served,
        });
        observation.effort = record.effort.map(|effort| strings.resolve(effort).into());
        observation.timestamp = record.timestamp.map(|time| time.get().into());
        if let Some(model) = observation.model.as_ref() {
            observation.invariants.push(("model", model.name));
        }
    }
    Ok(observation)
}

/// A message's response key: provider-scoped, or scoped to the recorded session when
/// conflicting responses share the message ID.
fn response_key(
    message: &MessageId,
    session: Sym,
    strings: &Strings,
    ambiguous: &HashSet<Digest>,
) -> Result<ScopedKey, AdapterError> {
    Ok(if ambiguous.contains(&message.digest) {
        AMBIGUOUS_RESPONSE_KEY.key(vec![
            KeyComponent::text(AGENT_NAMESPACE),
            KeyComponent::text(format!(
                "{}/{}",
                strings.resolve(session),
                strings.resolve(message.text)
            )),
        ])?
    } else {
        RESPONSE_KEY.key(vec![
            KeyComponent::text(PROVIDER_NAMESPACE),
            KeyComponent::text(strings.resolve(message.text)),
        ])?
    })
}

fn append_limits(
    record: &ParsedRecord,
    evidence: &EvidenceRef,
    strings: &Strings,
    known_threads: &HashMap<NativeThread, AnalyticalId>,
    ambiguous: &HashSet<Digest>,
    limits: &mut Vec<ProviderLimitObservation>,
) -> Result<(), AdapterError> {
    let Some(extras) = &record.extras else { return Ok(()) };
    if extras.quota.is_none() && extras.limit_text.is_none() {
        return Ok(());
    }
    let owner_thread = record
        .native_owner()
        .identity(strings)
        .ok()
        .map(|identity| identity.id)
        .filter(|id| known_threads.values().any(|known| known == id));
    let observed_at = record.timestamp.map(RecordTime::get).map_or(Basis::Unknown, Basis::Observed);
    if let Some(quota) = &extras.quota {
        let owner_request = record
            .message
            .as_ref()
            .map(|message| response_key(message, record.recorded_session(), strings, ambiguous))
            .transpose()?
            .map(|key| key.key.derive_id())
            .transpose()?;
        limits.push(ProviderLimitObservation {
            limit_name: quota.limit_name.as_deref().map(Name::new),
            window: None,
            observed_at: observed_at.clone(),
            owner_thread: owner_thread.clone(),
            owner_request,
            native: Name::new(&quota.native),
            evidence: *evidence,
        });
    }
    if let Some(message) = &extras.limit_text {
        limits.push(ProviderLimitObservation {
            limit_name: None,
            window: None,
            observed_at,
            owner_thread,
            owner_request: None,
            native: Name::new(&serde_json::json!({ "text": &**message }).to_string()),
            evidence: *evidence,
        });
    }
    Ok(())
}

fn cache_breakdown_mismatch(record: &ParsedRecord) -> Option<(u64, u64)> {
    let flat = record.count(Count::CacheWrite)?;
    let five = record.count(Count::CacheWrite5m);
    let hour = record.count(Count::CacheWrite1h);
    if five.is_none() && hour.is_none() {
        return None;
    }
    let breakdown = five.unwrap_or(0).checked_add(hour.unwrap_or(0))?;
    (flat != breakdown).then_some((flat, breakdown))
}

/// A record's total usage and, when advisor iterations add other models, its per-model
/// components. A single-model record has no components: its usage belongs to its model.
fn claude_usage(
    record: &ParsedRecord,
    strings: &Strings,
) -> Result<(TokenMeasures, Vec<ModelUsage>), AdapterError> {
    let input = record.count(Count::Input);
    let cache_read = record.count(Count::CacheRead);
    let flat_write = record.count(Count::CacheWrite);
    let five_minute_write = record.count(Count::CacheWrite5m);
    let one_hour_write = record.count(Count::CacheWrite1h);
    let output = record.count(Count::Output);
    let reasoning = record.count(Count::Reasoning);
    let has_breakdown = five_minute_write.is_some() || one_hour_write.is_some();
    let breakdown_total = five_minute_write.unwrap_or(0).checked_add(one_hour_write.unwrap_or(0));
    let breakdown_matches = match (flat_write, breakdown_total) {
        (Some(flat), Some(breakdown)) => flat == breakdown,
        (None, Some(_)) => true,
        (Some(_) | None, None) => false,
    };
    let primary = TokenMeasures {
        uncached_input: input,
        cache_read,
        cache_write_5m: (has_breakdown && breakdown_matches).then_some(five_minute_write).flatten(),
        cache_write_1h: (has_breakdown && breakdown_matches).then_some(one_hour_write).flatten(),
        cache_write_unspecified: (!has_breakdown || !breakdown_matches)
            .then_some(flat_write)
            .flatten(),
        output,
        reasoning,
        provider_only: None,
    };
    let advisors = record.advisors();
    if advisors.is_empty() {
        return Ok((primary, Vec::new()));
    }
    let mut usage = primary;
    let mut model_usage = vec![ModelUsage {
        model: record.model.map(|model| ModelName {
            name: strings.resolve(model).into(),
            basis: ModelBasis::Served,
        }),
        usage: primary.into(),
        source: "message.usage",
    }];
    for iteration in advisors {
        let advisor = TokenMeasures {
            uncached_input: iteration.input,
            cache_read: iteration.cache_read,
            cache_write_unspecified: iteration.cache_write,
            output: iteration.output,
            reasoning: iteration.reasoning,
            ..TokenMeasures::default()
        };
        usage = usage.checked_add(&advisor)?;
        model_usage.push(ModelUsage {
            model: iteration
                .model
                .as_ref()
                .map(|name| ModelName { name: name.as_str().into(), basis: ModelBasis::Served }),
            usage: advisor.into(),
            source: "advisor_message",
        });
    }
    Ok((usage, model_usage))
}

struct ClaudeBlockSelector;

impl RevisionSelector for ClaudeBlockSelector {
    fn rule(&self) -> &'static str {
        "claude-largest-output"
    }

    fn select(&self, revisions: &[&RequestObservation]) -> RevisionChoice {
        let selected = revisions
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| compare_claude_revision(left, right))
            .map_or(0, |(index, _)| index);
        let selected_usage = revisions[selected].usage.map(TokenMeasures::from);
        let disagreements = selected_usage.map_or_else(Vec::new, |selected| {
            revisions
                .iter()
                .filter_map(|revision| revision.usage.map(TokenMeasures::from))
                .any(|usage| input_measures(&usage) != input_measures(&selected))
                .then(|| "input or cache fields differ across Claude block records".to_owned())
                .into_iter()
                .collect()
        });
        RevisionChoice {
            selected,
            status: crate::ledger::entities::RevisionStatus::Selected,
            disagreements,
        }
    }
}

fn input_measures(measures: &TokenMeasures) -> [Option<u64>; 6] {
    [
        measures.uncached_input,
        measures.cache_read,
        measures.cache_write_5m,
        measures.cache_write_1h,
        measures.cache_write_unspecified,
        measures.provider_only,
    ]
}

fn compare_claude_revision(left: &RequestObservation, right: &RequestObservation) -> Ordering {
    let output = |observation: &RequestObservation| {
        observation.usage.and_then(|usage| TokenMeasures::from(usage).output).unwrap_or(0)
    };
    output(left)
        .cmp(&output(right))
        .then_with(|| left.sequence.cmp(&right.sequence))
        .then_with(|| left.evidence.offset.cmp(&right.evidence.offset))
        .then_with(|| right.evidence.source.cmp(&left.evidence.source))
}

fn thread_from_path(locator: &str, strings: &mut Strings) -> NativeThread {
    let path = Path::new(locator);
    let components: Vec<Cow<'_, str>> =
        path.components().map(|component| component.as_os_str().to_string_lossy()).collect();
    let subagents = components.iter().position(|component| component == "subagents");
    match subagents {
        Some(index) if index > 0 => {
            let session = strings.intern(&components[index.saturating_sub(1)]);
            let agent = path.file_stem().map(|stem| {
                let stem = stem.to_string_lossy();
                strings.intern(stem.strip_prefix("agent-").unwrap_or(&stem))
            });
            NativeThread { session, agent, inline_digest: None }
        }
        _ => NativeThread::main(strings.intern(
            &path.file_stem().map_or(Cow::Borrowed(locator), |stem| stem.to_string_lossy()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::fs;

    use serde_json::Value;

    use super::{
        ClaudeBlockSelector, Count, NativeThread, RecordTime, SourceDecoder, Strings, claude_usage,
        digest, read_subagent_meta_with_limit,
    };
    use crate::adapters::AdapterError;
    use crate::ledger::reconcile::{NativeSequence, RequestObservation, RevisionSelector};
    use crate::ledger::tokens::TokenMeasures;
    use crate::sources::evidence::EvidenceRef;
    use crate::sources::reader::{RawRecord, RecordDisposition};

    #[test]
    fn admission_is_shared_and_refuses_before_retaining_request_rows() {
        let budget = crate::sources::admission::Admission::new(1);
        let bytes =
            br#"{"type":"assistant","message":{"usage":{"input_tokens":3,"output_tokens":1}}}"#;
        let evidence = source_evidence(0);
        let raw = RawRecord { evidence: &evidence, bytes };
        let mut first = SourceDecoder::new("project/one.jsonl");
        let mut second = SourceDecoder::new("project/two.jsonl");
        assert_eq!(first.decode_with_admission(&raw, Some(&budget)), RecordDisposition::Decoded);
        assert_eq!(second.decode_with_admission(&raw, Some(&budget)), RecordDisposition::Stop);
        assert_eq!(first.records.len(), 1);
        assert!(second.records.is_empty());
        // Once any worker exhausts the budget, even content-only records cancel.
        let skipped = RawRecord { evidence: &evidence, bytes: b"{}" };
        assert_eq!(first.decode_with_admission(&skipped, Some(&budget)), RecordDisposition::Stop);
    }

    #[test]
    fn oversized_subagent_metadata_is_rejected_before_json_decode() {
        let root = tempfile::tempdir().unwrap();
        let transcript = root.path().join("agent-example.jsonl");
        fs::write(transcript.with_extension("meta.json"), vec![b' '; 33]).unwrap();

        let error = read_subagent_meta_with_limit(&transcript, 32).unwrap_err();

        assert!(matches!(error, AdapterError::MetadataTooLarge { maximum: 32, .. }));
    }

    fn source_evidence(offset: u64) -> EvidenceRef {
        EvidenceRef::new(0, offset, 1)
    }

    fn observation(offset: u64, block: u64) -> RequestObservation {
        let mut observation = RequestObservation::new(source_evidence(offset));
        observation.sequence = NativeSequence::new(block);
        observation.usage =
            Some(TokenMeasures { output: Some(10), ..TokenMeasures::default() }.into());
        observation
    }

    /// Decodes lines as one main-session transcript.
    fn decode(lines: &[Value]) -> (SourceDecoder, Vec<RecordDisposition>) {
        let mut decoder = SourceDecoder::new("project/session-one.jsonl");
        let mut offset = 0;
        let dispositions = lines
            .iter()
            .map(|line| {
                let bytes = serde_json::to_vec(line).unwrap();
                let evidence = source_evidence(offset);
                offset += 1;
                decoder.decode(&RawRecord { evidence: &evidence, bytes: &bytes })
            })
            .collect();
        (decoder, dispositions)
    }

    fn decode_one(line: &Value) -> SourceDecoder {
        let (decoder, dispositions) = decode(std::slice::from_ref(line));
        assert_eq!(dispositions, [RecordDisposition::Decoded]);
        decoder
    }

    #[test]
    fn block_index_breaks_equal_output_ties_before_file_position() {
        let later_offset = observation(20, 0);
        let later_block = observation(10, 1);
        let choice = ClaudeBlockSelector.select(&[&later_offset, &later_block]);
        assert_eq!(choice.selected, 1);
    }

    #[test]
    fn message_iterations_repeat_top_level_usage_without_adding_it() {
        let value = serde_json::json!({
            "type": "assistant",
            "message": {
                "model": "claude-test",
                "usage": {
                    "input_tokens": 3,
                    "output_tokens": 10,
                    "iterations": [{
                        "type": "message",
                        "input_tokens": 3,
                        "output_tokens": 10
                    }]
                }
            }
        });
        let decoder = decode_one(&value);
        let (usage, model_usage) = claude_usage(&decoder.records[0], &decoder.strings).unwrap();
        assert_eq!(usage.uncached_input, Some(3));
        assert_eq!(usage.output, Some(10));
        assert!(model_usage.is_empty());
    }

    #[test]
    fn advisor_iterations_add_usage_and_split_it_by_model() {
        let value = serde_json::json!({
            "type": "assistant",
            "message": {
                "model": "claude-test",
                "usage": {
                    "input_tokens": 3,
                    "output_tokens": 10,
                    "iterations": [
                        {
                            "type": "message",
                            "input_tokens": 3,
                            "output_tokens": 10
                        },
                        {
                            "type": "advisor_message",
                            "model": "claude-advisor",
                            "input_tokens": 5,
                            "output_tokens": 7
                        }
                    ]
                }
            }
        });
        let decoder = decode_one(&value);
        let (usage, model_usage) = claude_usage(&decoder.records[0], &decoder.strings).unwrap();

        assert_eq!(usage.uncached_input, Some(8));
        assert_eq!(usage.output, Some(17));
        assert_eq!(model_usage.len(), 2);
        assert_eq!(model_usage[0].model.as_ref().unwrap().name, "claude-test");
        assert_eq!(TokenMeasures::from(model_usage[0].usage).uncached_input, Some(3));
        assert_eq!(model_usage[1].model.as_ref().unwrap().name, "claude-advisor");
        assert_eq!(TokenMeasures::from(model_usage[1].usage).uncached_input, Some(5));
        assert_eq!(TokenMeasures::from(model_usage[1].usage).output, Some(7));
    }

    #[test]
    fn typed_records_drop_content_but_keep_tool_ownership_and_usage() {
        let payload = "x".repeat(2 * 1024 * 1024);
        let value = serde_json::json!({
            "type": "assistant",
            "uuid": "record-one",
            "message": {
                "id": "message-one",
                "model": "claude-test",
                "usage": {"input_tokens": 3, "output_tokens": 10},
                "content": [
                    {"type": "text", "text": payload},
                    {"type": "tool_use", "id": "tool-one", "input": {"prompt": payload}}
                ]
            }
        });

        let decoder = decode_one(&value);
        let record = &decoder.records[0];

        assert_eq!(
            record.message.as_ref().map(|message| decoder.strings.resolve(message.text)),
            Some("message-one")
        );
        assert_eq!(record.uuid, Some(digest("record-one")));
        assert_eq!(decoder.tool_uses, [(digest("tool-one"), decoder.thread)]);
        assert_eq!(record.count(Count::Input), Some(3));
        assert_eq!(record.count(Count::Output), Some(10));
        assert_eq!(record.count(Count::CacheRead), None);
        assert!(record.extras.is_none());
    }

    #[test]
    fn only_usage_lines_become_records_and_every_line_feeds_source_facts() {
        let (decoder, dispositions) = decode(&[
            serde_json::json!({"type": "user", "cwd": "/work/app", "version": "2.1.0"}),
            serde_json::json!({"type": "assistant", "message": {"usage": {"input_tokens": 1}}}),
            serde_json::json!({"type": "progress", "data": {"message": {"type": "assistant"}}}),
            serde_json::json!({
                "type": "progress",
                "sessionId": "outer",
                "data": {"message": {
                    "sessionId": "inner",
                    "message": {"usage": {"output_tokens": 2}}
                }}
            }),
            serde_json::json!({"type": "user", "isSidechain": true}),
        ]);
        let decoder_output = &decoder.records;

        assert_eq!(
            dispositions,
            [
                RecordDisposition::Skipped,
                RecordDisposition::Skipped,
                RecordDisposition::Skipped,
                RecordDisposition::Decoded,
                RecordDisposition::Skipped,
            ]
        );
        assert_eq!(decoder.facts.version.as_deref(), Some("2.1.0"));
        assert_eq!(
            decoder.facts.project.map(|project| decoder.strings.resolve(project)),
            Some("app")
        );
        assert_eq!(decoder_output.len(), 1);
        let nested = &decoder_output[0];
        assert!(nested.forced_copy && nested.request_record);
        assert_eq!(nested.session.map(|session| decoder.strings.resolve(session)), Some("outer"));
        assert_eq!(decoder.facts.inline_threads.len(), 1);
        assert_eq!(
            decoder.facts.last_main_evidence.as_ref().map(|evidence| evidence.offset),
            Some(3)
        );
    }

    #[test]
    fn a_malformed_line_is_malformed_and_still_starts_the_source() {
        let mut decoder = SourceDecoder::new("project/session-one.jsonl");
        let evidence = source_evidence(0);
        let disposition = decoder.decode(&RawRecord {
            evidence: &evidence,
            bytes: b"{\"type\":\"assistant\",\"n\":1e400}",
        });
        assert_eq!(disposition, RecordDisposition::Malformed);
        assert_eq!(decoder.facts.evidence, Some(evidence));
    }

    #[test]
    fn record_times_round_trip_every_sub_second_sign() {
        for (second, nanosecond) in
            [(0, 0), (1_700_000_000, 999_999_999), (-1, -1), (-86_400, -500_000_000), (5, 1)]
        {
            let timestamp = jiff::Timestamp::new(second, nanosecond).unwrap();
            assert_eq!(RecordTime::new(timestamp).get(), timestamp);
        }
        for timestamp in [jiff::Timestamp::MIN, jiff::Timestamp::MAX] {
            assert_eq!(RecordTime::new(timestamp).get(), timestamp);
        }
        assert_eq!(size_of::<Option<RecordTime>>(), 16);
    }

    #[test]
    fn merged_symbols_follow_discovery_order_and_compare_as_strings() {
        let mut first = Strings::default();
        let a = NativeThread { session: first.intern("b"), agent: None, inline_digest: None };
        let mut second = Strings::default();
        let b = NativeThread {
            session: second.intern("a"),
            agent: Some(second.intern("b")),
            inline_digest: None,
        };

        first.freeze();
        second.freeze();
        let mut merged = Strings::default();
        let a = merged.absorb(first).thread(a);
        let b = merged.absorb(second).thread(b);

        assert_eq!(a.session, b.agent.unwrap(), "one string, one symbol");
        assert_eq!(merged.resolve(a.session), "b");
        assert_eq!(a.text_order(b, &merged), Ordering::Greater, "\"b\" sorts after \"a\"");
        let main_a = NativeThread::main(b.session);
        assert_eq!(main_a.text_order(b, &merged), Ordering::Less, "no agent sorts first");
    }
}
