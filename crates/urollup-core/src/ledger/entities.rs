//! Normalized ledger entities (design §3.1).
//!
//! Every property value records its [`Basis`]: observed in a record, configured by a rule,
//! inferred with a stated method, or unknown. Model and effort belong to requests, and
//! ownership is explicit: a request is owned, ambiguous among candidates, or unknown.
//!
//! Provider charges, resource observations and annotations are later entities and are not
//! defined yet.

use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;

use super::identity::{AnalyticalId, StoredIdentity};
use super::scope::IdentityBasis;
use super::tokens::TokenMeasures;
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
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelName {
    /// The native model name.
    pub name: String,
    /// Served or requested.
    pub basis: ModelBasis,
}

/// One model's contribution to a request's usage.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelUsage {
    /// The model, when the source identifies it.
    pub model: Option<ModelName>,
    /// The usage attributed to this model.
    pub usage: TokenMeasures,
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

/// One usage-bearing record of a request.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UsageRevision {
    /// The record.
    pub evidence: EvidenceRef,
    /// Its usage.
    pub usage: TokenMeasures,
    /// The usage split by model, recorded only when one request invokes more than one
    /// model; a single-model request's usage belongs to its own model.
    pub model_usage: Vec<ModelUsage>,
}

/// The usage revision a request counts.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectedUsage {
    /// The selected revision.
    pub revision: UsageRevision,
    /// Final or selected by rule.
    pub status: RevisionStatus,
    /// The selector's rule name.
    pub rule: &'static str,
}

/// A request's account attribution; conflicts are diagnosed, never split.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AccountAttribution {
    /// One stable account identifier.
    Attributed(String),
    /// Observations name several accounts.
    Conflicting(BTreeSet<String>),
    /// No account is recorded.
    Unknown,
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

/// A logical request and its response (design §3.1).
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
    pub first_seen: Option<Timestamp>,
    /// Latest timestamp among original records.
    pub last_seen: Option<Timestamp>,
    /// The model, when recorded.
    pub model: Option<ModelName>,
    /// The reasoning effort, when recorded.
    pub effort: Option<String>,
    /// The account attribution.
    pub account: AccountAttribution,
    /// The counted usage revision; `None` when no original record carries usage.
    pub usage: Option<SelectedUsage>,
    /// Every original record, in canonical order.
    pub evidence: Box<[EvidenceRef]>,
    /// Copies of this request, recorded as evidence and never counted.
    pub copies: Box<[EvidenceRef]>,
    /// Whether the request counts in totals.
    pub counting: Counting,
}

impl Request {
    /// The canonical ID.
    pub fn id(&self) -> &AnalyticalId {
        &self.id
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
    pub limit_name: Option<String>,
    /// The native window label, such as Codex `primary` or `secondary`.
    pub window: Option<String>,
    /// When the limit was observed; unknown for sources without a record timestamp.
    pub observed_at: Basis<Timestamp>,
    /// The owning thread, when proven.
    pub owner_thread: Option<AnalyticalId>,
    /// The owning request, when proven.
    pub owner_request: Option<AnalyticalId>,
    /// Native field names and values verbatim, including window length, reset time,
    /// utilization in its native unit, plan, credit and overage fields, as one compact
    /// JSON object with sorted keys.
    pub native: Box<str>,
    /// The record.
    pub evidence: EvidenceRef,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::apply_permutation;

    proptest! {
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
