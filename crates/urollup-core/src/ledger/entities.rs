//! Normalized ledger entities (design §3.1).
//!
//! Every property value records its [`Basis`]: observed in a record, configured by a rule,
//! inferred with a stated method, or unknown. Model and effort belong to requests, and
//! ownership is explicit: a request is owned, ambiguous among candidates, or unknown.
//!
//! Provider charges, resource observations and annotations are later entities and are not
//! defined yet.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroU32;
use std::slice;

use jiff::Timestamp;

use super::identity::{AnalyticalId, StoredIdentity};
use super::inline_list::InlineList;
use super::names::Name;
use super::scope::IdentityBasis;
use super::tokens::Measures;
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::ManifestEntry;

/// A value with the basis it was established on.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Basis<T> {
    /// Recorded in a source record.
    Observed(T),
    /// Set by a declared, versioned rule.
    Configured(T),
    /// Derived by a stated inference.
    Inferred(T),
    /// Not established.
    Unknown,
}

impl<T> Basis<T> {
    /// The value, whatever its basis; `None` when unknown.
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Observed(value) | Self::Configured(value) | Self::Inferred(value) => Some(value),
            Self::Unknown => None,
        }
    }

    /// The contract token: `observed`, `configured`, `inferred` or `unknown`.
    pub const fn token(&self) -> &'static str {
        match self {
            Self::Observed(_) => "observed",
            Self::Configured(_) => "configured",
            Self::Inferred(_) => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

/// A source artifact: a snapshot entry with its dialect facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceArtifact {
    /// The dialect version the records declare, such as a CLI version.
    pub dialect_version: Basis<String>,
    /// Whether the adapter can read this source.
    pub capability: SourceCapability,
    /// The snapshot manifest entry: identity, fingerprint, extent, counters and changes.
    pub snapshot: ManifestEntry,
}

/// Whether an adapter supports a source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceCapability {
    /// The adapter reads this source.
    Supported,
    /// The source is recognized but unsupported, with the reason.
    Unsupported(String),
}

/// A thread: one agent conversation or subagent run.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Thread {
    /// The canonical `thr-` ID with its key.
    pub identity: StoredIdentity,
    /// The identity basis.
    pub basis: IdentityBasis,
    /// Other IDs linked to this thread, with their keys.
    pub aliases: Vec<StoredIdentity>,
    /// The native thread key fields verbatim, such as session ID and subagent ID.
    pub native_key: BTreeMap<String, String>,
    /// How the thread was started, such as `cli`, `exec` or `subagent`.
    pub source: Basis<String>,
    /// Who initiated it, such as a user or a parent agent.
    pub initiator: Basis<String>,
    /// Its purpose, such as a recorded subagent type or review mode.
    pub purpose: Basis<String>,
    /// Where it ran, such as a local host or a cloud sandbox.
    pub execution_environment: Basis<String>,
    /// The project, a plain name and never a path.
    pub project: Basis<String>,
    /// The stable account identifier.
    pub account: Basis<String>,
    /// The records that establish the thread.
    pub evidence: Vec<EvidenceRef>,
}

/// A native edge between threads.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelationshipKind {
    /// A subagent spawned by a parent.
    Spawn,
    /// A fork over copied history.
    Fork,
    /// A resumed thread.
    Resume,
    /// A review thread.
    Review,
    /// An older inline sidechain turn that became a child thread.
    InlineSidechain,
    /// Another native edge, named by its registry token.
    Other(String),
}

impl RelationshipKind {
    /// Whether the edge adds the child to `descendants` scope. Spawn and inline sidechain
    /// edges do; fork edges never do, because copied history is deduplicated rather than
    /// owned twice (design §3.2).
    pub const fn defines_descendants(&self) -> bool {
        match self {
            Self::Spawn | Self::InlineSidechain => true,
            Self::Fork | Self::Resume | Self::Review | Self::Other(_) => false,
        }
    }
}

/// How firmly an edge or link is established.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Confidence {
    /// A native field proves it.
    Proven,
    /// It is inferred, with a diagnostic stating how.
    Inferred,
}

/// A relationship between two threads, keyed by kind and endpoint IDs.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Relationship {
    /// The edge kind.
    pub kind: RelationshipKind,
    /// The parent or origin thread.
    pub from: AnalyticalId,
    /// The child or destination thread.
    pub to: AnalyticalId,
    /// How firmly the edge is established.
    pub confidence: Confidence,
    /// The records that establish it.
    pub evidence: Vec<EvidenceRef>,
}

/// Which thread owns a logical request (design §4.2).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Ownership {
    /// One thread is proven to own it.
    Owned {
        /// The owner.
        thread: AnalyticalId,
    },
    /// Several candidate threads may own it, or one thread holds it without proof.
    Ambiguous {
        /// The candidates.
        candidates: BTreeSet<AnalyticalId>,
    },
    /// No owner evidence exists.
    Unknown,
}

impl Ownership {
    /// The contract token: `owned`, `ambiguous` or `unknown`.
    pub const fn token(&self) -> &'static str {
        match self {
            Self::Owned { .. } => "owned",
            Self::Ambiguous { .. } => "ambiguous",
            Self::Unknown => "unknown",
        }
    }
}

/// Where a request's model name comes from.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ModelBasis {
    /// The response records the model that served it.
    Served,
    /// Only the request records the model asked for.
    Requested,
}

/// A model name with its basis. Placeholder names stay as observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelName {
    /// The native model name.
    pub name: Name,
    /// Served or requested.
    pub basis: ModelBasis,
}

/// One model's contribution to a request's usage.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelUsage {
    /// The model, when the source identifies it.
    pub model: Option<ModelName>,
    /// The usage attributed to this model.
    pub usage: Measures,
    /// The stable dialect field or record kind that carried this component.
    pub source: &'static str,
}

/// Whether a request's selected usage is its final revision.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RevisionStatus {
    /// The dialect orders its revisions, and this is the last.
    Final,
    /// The dialect cannot order its revisions; its selection rule picked this one.
    Selected,
}

/// A list of per-model usage components, empty for almost every record, so it is one
/// pointer that allocates only when a component exists.
pub type ModelUsageList = InlineList<ModelUsage, 0>;

/// The usage of one usage-bearing record of a request.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UsageRevision {
    /// Its usage.
    pub usage: Measures,
    /// The usage split by model, recorded only when one request invokes more than one
    /// model; a single-model request's usage belongs to its own model.
    pub model_usage: ModelUsageList,
}

/// The usage revision a request counts.
///
/// The selector's rule name is the same for every request of a ledger, so the ledger
/// records it once as [`Ledger::revision_rule`](super::reconcile::Ledger::revision_rule).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectedUsage {
    /// The selected revision.
    pub revision: UsageRevision,
    /// The selected record's position in [`Request::evidence`], which lists every
    /// original record and so every revision.
    pub evidence: u32,
    /// Final or selected by rule.
    pub status: RevisionStatus,
}

/// Whether a request's usage counts in totals.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Counting {
    /// Counted once in default totals.
    Counted,
    /// A non-counted member of a candidate set, reported as unresolved usage.
    Unresolved {
        /// The member the candidate set counts instead.
        counted: AnalyticalId,
    },
    /// Observed only as copies, whose usage never counts.
    CopyOnly,
}

/// A timestamp in 12 bytes: whole seconds and biased nanoseconds, whose niche keeps an
/// optional one at 12 bytes too, where `Option<Timestamp>` takes 24.
///
/// Its order is the timestamp's order: sub-second nanoseconds share the sign of the
/// seconds, so comparing seconds and then nanoseconds orders instants.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct CompactTimestamp {
    /// Seconds since the Unix epoch as native-endian bytes, which need no alignment.
    second: [u8; 8],
    /// Sub-second nanoseconds plus one billion, which is never zero.
    biased_nanosecond: NonZeroU32,
}

const _: () = assert!(std::mem::size_of::<Option<CompactTimestamp>>() == 12);

impl CompactTimestamp {
    const BIAS: i32 = 1_000_000_000;

    /// The timestamp.
    pub fn get(self) -> Timestamp {
        let nanosecond = i32::try_from(self.biased_nanosecond.get())
            .map_or(0, |biased| biased.saturating_sub(Self::BIAS));
        Timestamp::new(self.second(), nanosecond).expect("a decomposed timestamp recomposes")
    }

    const fn second(self) -> i64 {
        i64::from_ne_bytes(self.second)
    }
}

impl From<Timestamp> for CompactTimestamp {
    fn from(timestamp: Timestamp) -> Self {
        let biased = timestamp.subsec_nanosecond().saturating_add(Self::BIAS);
        Self {
            second: timestamp.as_second().to_ne_bytes(),
            biased_nanosecond: u32::try_from(biased)
                .ok()
                .and_then(NonZeroU32::new)
                .expect("sub-second nanoseconds lie strictly within one second"),
        }
    }
}

impl From<CompactTimestamp> for Timestamp {
    fn from(timestamp: CompactTimestamp) -> Self {
        timestamp.get()
    }
}

impl Ord for CompactTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.second(), self.biased_nanosecond).cmp(&(other.second(), other.biased_nanosecond))
    }
}

impl PartialOrd for CompactTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for CompactTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

/// Originals then copies for one request.
///
/// The common case is one original and no copies, stored inline so a whole-history
/// ledger does not allocate once per request. Two or more refs share one boxed slice.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RecordRefs {
    /// No original or copy records.
    Empty,
    /// One original or one copy.
    One(EvidenceRef),
    /// Two or more refs, originals then copies.
    Many(Box<[EvidenceRef]>),
}

impl RecordRefs {
    fn as_slice(&self) -> &[EvidenceRef] {
        match self {
            Self::Empty => &[],
            Self::One(record) => slice::from_ref(record),
            Self::Many(records) => records,
        }
    }

    fn split(&self, originals: u32) -> (&[EvidenceRef], &[EvidenceRef]) {
        let records = self.as_slice();
        usize::try_from(originals)
            .ok()
            .and_then(|originals| records.split_at_checked(originals))
            .unwrap_or((records, &[]))
    }
}

impl FromIterator<EvidenceRef> for RecordRefs {
    fn from_iter<I: IntoIterator<Item = EvidenceRef>>(items: I) -> Self {
        let mut items = items.into_iter();
        let Some(first) = items.next() else {
            return Self::Empty;
        };
        let Some(second) = items.next() else {
            return Self::One(first);
        };
        let mut records = vec![first, second];
        records.extend(items);
        Self::Many(records.into_boxed_slice())
    }
}

const _: () = assert!(std::mem::size_of::<RecordRefs>() <= 24);

/// A logical request and its response (design §3.1).
///
/// Whole-history ledgers hold hundreds of thousands of requests, so names are interned,
/// usage and time are stored compactly, and a single evidence ref stays inline.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Request {
    /// The canonical `req-` ID.
    pub id: AnalyticalId,
    /// The identity basis.
    pub basis: IdentityBasis,
    /// Other IDs linked to this request.
    pub aliases: Box<[AnalyticalId]>,
    /// Owner or candidates.
    pub ownership: Ownership,
    /// Earliest timestamp among original records.
    pub first_seen: Option<CompactTimestamp>,
    /// Latest timestamp among original records.
    pub last_seen: Option<CompactTimestamp>,
    /// The model, when recorded.
    pub model: Option<ModelName>,
    /// The reasoning effort, when recorded.
    pub effort: Option<Name>,
    /// The counted usage revision; `None` when no original record carries usage.
    pub usage: Option<SelectedUsage>,
    /// Every original record in canonical order, then every copy in canonical order.
    pub records: RecordRefs,
    /// How many of `records` are originals.
    pub originals: u32,
    /// Whether the request counts in totals.
    pub counting: Counting,
}

// Compact Measures shrinks the selected-usage revision.
const _: () = assert!(std::mem::size_of::<Request>() <= 216);

impl Request {
    /// The canonical ID.
    pub fn id(&self) -> &AnalyticalId {
        &self.id
    }

    /// Every original record, in canonical order.
    pub fn evidence(&self) -> &[EvidenceRef] {
        self.split_records().0
    }

    /// Copies of this request, recorded as evidence and never counted.
    pub fn copies(&self) -> &[EvidenceRef] {
        self.split_records().1
    }

    /// The record of the counted usage revision.
    pub fn selected_evidence(&self) -> Option<&EvidenceRef> {
        let selected = self.usage.as_ref()?;
        self.evidence().get(usize::try_from(selected.evidence).ok()?)
    }

    fn split_records(&self) -> (&[EvidenceRef], &[EvidenceRef]) {
        self.records.split(self.originals)
    }
}

/// Logical requests sorted by canonical ID.
///
/// A sorted vector rather than a map: a whole-history ledger holds hundreds of thousands
/// of requests, and tree nodes would more than double their footprint.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Requests {
    rows: Vec<Request>,
}

impl Requests {
    /// Sorts requests by ID. IDs must be distinct; for a repeated ID the last request
    /// given wins, as inserting into a map would.
    pub fn from_unsorted(mut rows: Vec<Request>) -> Self {
        // Sort a permutation and apply it in place: a stable sort of wide rows would
        // allocate a buffer of half the table. Among equal IDs the latest comes first.
        let mut order: Vec<usize> = (0..rows.len()).collect();
        order.sort_unstable_by(|&left, &right| {
            rows[left].id.cmp(&rows[right].id).then(right.cmp(&left))
        });
        apply_permutation(&mut rows, &mut order);
        rows.dedup_by(|later, earlier| later.id == earlier.id);
        if rows.capacity() - rows.len() > rows.len() / 16 {
            rows.shrink_to_fit();
        }
        Self { rows }
    }

    fn position(&self, id: &AnalyticalId) -> Option<usize> {
        self.rows.binary_search_by(|request| request.id.cmp(id)).ok()
    }

    /// The request with this canonical ID.
    pub fn get(&self, id: &AnalyticalId) -> Option<&Request> {
        self.position(id).and_then(|index| self.rows.get(index))
    }

    /// The request with this canonical ID, mutably.
    pub fn get_mut(&mut self, id: &AnalyticalId) -> Option<&mut Request> {
        self.position(id).and_then(|index| self.rows.get_mut(index))
    }

    /// Whether a request has this canonical ID.
    pub fn contains_key(&self, id: &AnalyticalId) -> bool {
        self.position(id).is_some()
    }

    /// The number of requests.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether there are no requests.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Requests in ID order.
    pub fn values(&self) -> std::slice::Iter<'_, Request> {
        self.rows.iter()
    }

    /// Canonical IDs in order.
    pub fn keys(&self) -> impl Iterator<Item = &AnalyticalId> {
        self.rows.iter().map(|request| &request.id)
    }

    /// Requests with their IDs, in ID order.
    pub fn iter(&self) -> impl Iterator<Item = (&AnalyticalId, &Request)> {
        self.rows.iter().map(|request| (&request.id, request))
    }
}

/// Reorders `rows` so position `k` holds the row that was at `order[k]`, by swapping along
/// the permutation's cycles; `order` is left as the identity.
fn apply_permutation<T>(rows: &mut [T], order: &mut [usize]) {
    for start in 0..rows.len() {
        let mut current = start;
        while order[current] != current {
            let source = order[current];
            order[current] = current;
            if source == start {
                break;
            }
            rows.swap(current, source);
            current = source;
        }
    }
}

impl std::ops::Index<&AnalyticalId> for Requests {
    type Output = Request;

    fn index(&self, id: &AnalyticalId) -> &Request {
        self.get(id).expect("no request has this ID")
    }
}

/// A tool action, linked to a request only when proven.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ToolAction {
    /// The canonical `act-` ID with its key.
    pub identity: StoredIdentity,
    /// The identity basis.
    pub basis: IdentityBasis,
    /// The native call ID.
    pub call_id: Option<String>,
    /// The native tool name.
    pub tool_name: Option<String>,
    /// The request that made the call, only when a native field proves it.
    pub request: Option<AnalyticalId>,
    /// The call and result records.
    pub evidence: Vec<EvidenceRef>,
}

/// A usage-limit record as the source wrote it (design §3.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderLimitObservation {
    /// The native limit name, such as a Codex `limit_id` or Claude `rateLimitType`.
    pub limit_name: Option<Name>,
    /// The native window label, such as Codex `primary` or `secondary`.
    pub window: Option<Name>,
    /// When the limit was observed; unknown for sources without a record timestamp.
    pub observed_at: Basis<Timestamp>,
    /// The owning thread, when proven.
    pub owner_thread: Option<AnalyticalId>,
    /// The owning request, when proven.
    pub owner_request: Option<AnalyticalId>,
    /// Native field names and values verbatim, including window length, reset time,
    /// utilization in its native unit, plan, credit and overage fields, as one interned
    /// compact JSON object with sorted keys.
    pub native: Name,
    /// The record.
    pub evidence: EvidenceRef,
}

const _: () = assert!(std::mem::size_of::<ProviderLimitObservation>() <= 128);

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use proptest::prelude::*;

    use crate::sources::evidence::EvidenceRef;

    use super::{CompactTimestamp, RecordRefs, apply_permutation};

    /// Timestamps across jiff's whole range, and near the epoch with sub-second parts of
    /// either sign, which jiff normalizes to the sign of the seconds.
    fn arbitrary_timestamp() -> impl Strategy<Value = Timestamp> {
        let range = Timestamp::MIN.as_nanosecond()..=Timestamp::MAX.as_nanosecond();
        prop_oneof![
            range.prop_map(|nanosecond| Timestamp::from_nanosecond(nanosecond).unwrap()),
            (-3_i64..3, -999_999_999_i32..=999_999_999)
                .prop_map(|(second, nanosecond)| Timestamp::new(second, nanosecond).unwrap()),
        ]
    }

    #[test]
    fn record_refs_keep_one_ref_inline_and_spill_the_rest() {
        let a = EvidenceRef::new(0, 1, 2);
        let b = EvidenceRef::new(0, 3, 4);
        let c = EvidenceRef::new(1, 5, 6);
        assert_eq!(RecordRefs::from_iter([]), RecordRefs::Empty);
        assert_eq!(RecordRefs::from_iter(std::iter::once(a)), RecordRefs::One(a));
        assert_eq!(RecordRefs::from_iter([a, b]), RecordRefs::Many(vec![a, b].into_boxed_slice()));
        assert_eq!(RecordRefs::One(a).split(1), (&[a][..], &[][..]));
        assert_eq!(RecordRefs::One(a).split(0), (&[][..], &[a][..]));
        assert_eq!(RecordRefs::from_iter([a, b, c]).split(1), (&[a][..], &[b, c][..]));
        assert!(std::mem::size_of::<RecordRefs>() <= 24);
        assert!(std::mem::size_of::<super::Request>() <= 216);
    }

    proptest! {
        #[test]
        fn compact_timestamps_round_trip_and_order_like_timestamps(
            left in arbitrary_timestamp(),
            right in arbitrary_timestamp(),
        ) {
            let (compact_left, compact_right) =
                (CompactTimestamp::from(left), CompactTimestamp::from(right));
            prop_assert_eq!(compact_left.get(), left);
            prop_assert_eq!(compact_left.cmp(&compact_right), left.cmp(&right));
            prop_assert_eq!(compact_left == compact_right, left == right);
        }

        #[test]
        fn applying_a_sorting_permutation_matches_sorting(
            rows in prop::collection::vec(0u16..50, 0..64),
        ) {
            let mut order: Vec<usize> = (0..rows.len()).collect();
            order.sort_by_key(|&index| (rows[index], std::cmp::Reverse(index)));
            let expected: Vec<(u16, usize)> = order.iter().map(|&index| (rows[index], index)).collect();
            let mut tagged: Vec<(u16, usize)> = rows.iter().copied().zip(0..).collect();
            apply_permutation(&mut tagged, &mut order);
            prop_assert_eq!(tagged, expected);
            prop_assert!(order.iter().enumerate().all(|(position, value)| position == *value));
        }
    }
}
