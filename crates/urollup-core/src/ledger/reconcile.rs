//! The generic reconciliation engine: observations to logical entities (design §3.3).
//!
//! Adapters emit normalized observations; [`reconcile`] merges them into one ledger
//! before any aggregation. Every step uses sorted data so the ledger never depends on the
//! order files were read or observations arrived. Threads and tool actions merge by
//! analytical identity, relationships by kind and endpoints, and provider limit records
//! preserve changes while collapsing identical consecutive snapshots. Request
//! reconciliation then follows these steps:
//!
//! 1. **Re-reads:** observations are sorted into canonical order (evidence first).
//!    Identical observations are one record read twice and count once. Two observations
//!    of one record location with different content get a
//!    [`DiagnosticCode::ConflictingReread`] and the first in canonical order is kept.
//! 2. **Identity:** every key's ID is registered with further digest bits, so two keys
//!    yielding one ID fail with [`ReconcileError::Identity`]. An observation without a
//!    usable key gets its artifact-local ID. Observations sharing any key ID, or joined by
//!    a [`LineageLink`], form one linked set; sets are found by sorting keys by ID.
//! 3. **Conflicting shared keys:** when a linked set's observations disagree on a
//!    revision-invariant field, the set is not merged: each observation becomes its own
//!    ambiguous request under its artifact-local ID, with a
//!    [`DiagnosticCode::ConflictingSharedKey`], and the requests form one candidate set.
//! 4. **Requests:** a linked set takes the ID of its highest-precedence key, then the
//!    lowest ID. Original observations with usage are its revisions, passed in canonical
//!    order to the dialect's [`RevisionSelector`]; copies are recorded as evidence and
//!    never counted, and a request seen only as copies is [`Counting::CopyOnly`].
//!    Ownership comes from proven owners; conflicting owners and served models are
//!    diagnosed, not split.
//! 5. **Candidate sets:** requests split from one conflicting key form a candidate set. It
//!    counts the member with the strongest identity basis, then the lowest ID, and marks
//!    the others [`Counting::Unresolved`], which totals never add.
//!
//! Observations are stored in chunks and moved into set order, and requests are built from
//! the last set backwards into chunks while each finished set's observations are freed.
//! Request chunks reuse the freed observation chunks, so a whole-history run's footprint
//! holds about one of the two row tables rather than both.

use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;

use self::grouping::Grouping;
use super::chunked::ChunkedVec;
use super::coverage::{CoverageGap, ReconcileCoverage};
use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::entities::{
    Basis, CompactTimestamp, Confidence, Counting, ModelBasis, ModelName, ModelUsageList,
    Ownership, ProviderLimitObservation, Relationship, RelationshipKind, Request, Requests,
    RevisionStatus, SelectedUsage, Thread, ToolAction, UsageRevision,
};
use super::identity::{AnalyticalId, IdPrefix, IdentityError, IdentityRegistry};
use super::inline_list::InlineList;
use super::linking::LinkGraph;
use super::names::Name;
use super::scope::{DerivedKey, IdentityBasis, artifact_local_key};
use super::tokens::Measures;
use crate::sources::evidence::EvidenceRef;

/// Whether an observation is the request's own record or a copy of it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObservationRole {
    /// The request's own record in its owner's source.
    Original,
    /// A replay nested in or copied into another record or file; never counted.
    Copy,
}

/// What an observation says about the owning thread.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OwnerEvidence {
    /// A native field proves this thread owns the request.
    Proven(AnalyticalId),
    /// The record says nothing about ownership.
    None,
}

/// One request record as an adapter decoded it.
///
/// The derived order starts with the evidence reference, which makes it the canonical
/// observation order. Names are interned and usage and time are stored compactly, because
/// a whole-history run holds hundreds of thousands of observations at once.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestObservation {
    /// Where the record is.
    pub evidence: EvidenceRef,
    /// Every `req-` key the adapter could build for the record, in any order.
    pub keys: InlineList<DerivedKey, 2>,
    /// Original or copy.
    pub role: ObservationRole,
    /// Owner evidence.
    pub owner: OwnerEvidence,
    /// The record's usage, when it carries any.
    pub usage: Option<Measures>,
    /// This record's usage split by model.
    pub model_usage: ModelUsageList,
    /// A native revision sequence, when the dialect orders revisions.
    pub sequence: Option<u64>,
    /// Revision-invariant fields and their interned values; observations sharing a key
    /// must agree on every field both carry.
    pub invariants: InlineList<(&'static str, Name), 1>,
    /// The model, when recorded.
    pub model: Option<ModelName>,
    /// The reasoning effort, when recorded.
    pub effort: Option<Name>,
    /// The record's timestamp.
    pub timestamp: Option<CompactTimestamp>,
}

// 288 bytes, down from 448. The evidence reference (40 bytes), two inline keys (64) and the
// measures (72) take 176; a smaller row needs evidence that names its source by index.
const _: () = assert!(std::mem::size_of::<RequestObservation>() <= 288);

impl RequestObservation {
    /// An original observation with no keys, owner, usage or properties yet.
    pub fn new(evidence: EvidenceRef) -> Self {
        Self {
            evidence,
            keys: InlineList::new(),
            role: ObservationRole::Original,
            owner: OwnerEvidence::None,
            usage: None,
            model_usage: InlineList::new(),
            sequence: None,
            invariants: InlineList::new(),
            model: None,
            effort: None,
            timestamp: None,
        }
    }
}

/// Lineage evidence that two request IDs are one request, such as a fork edge over copied
/// history.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LineageLink {
    /// One request ID.
    pub a: AnalyticalId,
    /// The other request ID.
    pub b: AnalyticalId,
    /// The records that establish the link.
    pub evidence: Vec<EvidenceRef>,
}

/// A selector's choice among a request's revisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionChoice {
    /// Index into the revisions slice passed to [`RevisionSelector::select`].
    pub selected: usize,
    /// Final when the dialect orders revisions, else selected by rule.
    pub status: RevisionStatus,
    /// Disagreements among revisions worth a diagnostic, such as differing input fields
    /// when a dialect selects by output.
    pub disagreements: Vec<String>,
}

/// A dialect's rule for choosing which usage revision a request counts (design §3.4).
///
/// Implementations must be deterministic functions of the slice, which reconciliation
/// passes in canonical evidence order and never empty.
pub trait RevisionSelector {
    /// A stable rule name, recorded once per ledger as [`Ledger::revision_rule`].
    fn rule(&self) -> &'static str;

    /// Chooses one revision.
    fn select(&self, revisions: &[&RequestObservation]) -> RevisionChoice;
}

/// The default rule: the highest native sequence when every revision has one (final),
/// else the last revision in canonical evidence order (selected by rule).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LatestRevision;

impl RevisionSelector for LatestRevision {
    fn rule(&self) -> &'static str {
        "latest-revision"
    }

    fn select(&self, revisions: &[&RequestObservation]) -> RevisionChoice {
        let last = revisions.len().saturating_sub(1);
        let sequences: Option<Vec<u64>> =
            revisions.iter().map(|revision| revision.sequence).collect();
        match sequences {
            Some(sequences) => {
                // `max_by_key` keeps the last maximum, so ties resolve to canonical order.
                let selected = sequences
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, sequence)| **sequence)
                    .map_or(last, |(index, _)| index);
                RevisionChoice {
                    selected,
                    status: RevisionStatus::Final,
                    disagreements: Vec::new(),
                }
            }
            None => RevisionChoice {
                selected: last,
                status: if revisions.len() > 1 {
                    RevisionStatus::Selected
                } else {
                    RevisionStatus::Final
                },
                disagreements: Vec::new(),
            },
        }
    }
}

/// Everything one reconciliation merges.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReconcileInput {
    /// Thread observations, in any order.
    pub threads: Vec<Thread>,
    /// Relationship observations, in any order.
    pub relationships: Vec<Relationship>,
    /// Request observations, in any order, in chunks that reconciliation frees as it builds
    /// requests.
    pub requests: ChunkedVec<RequestObservation>,
    /// Tool action observations, in any order.
    pub tool_actions: Vec<ToolAction>,
    /// Provider limit observations, in any order.
    pub limit_observations: Vec<ProviderLimitObservation>,
    /// Lineage links between request IDs.
    pub links: Vec<LineageLink>,
    /// Unobserved coverage gaps.
    pub gaps: Vec<CoverageGap>,
    /// Diagnostics adapters raised while decoding, carried into the ledger.
    pub diagnostics: Vec<Diagnostic>,
}

/// The reconciled request ledger.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Ledger {
    /// Logical threads by canonical ID.
    pub threads: BTreeMap<AnalyticalId, Thread>,
    /// Native relationships in canonical order.
    pub relationships: Vec<Relationship>,
    /// Logical requests by canonical ID.
    pub requests: Requests,
    /// Logical tool actions by canonical ID.
    pub tool_actions: BTreeMap<AnalyticalId, ToolAction>,
    /// Provider limit observations in canonical order.
    pub limit_observations: Vec<ProviderLimitObservation>,
    /// Candidate sets with more than one member.
    pub candidate_sets: Vec<BTreeSet<AnalyticalId>>,
    /// The rule name of the selector that chose every request's counted usage.
    pub revision_rule: &'static str,
    /// Diagnostics in canonical order.
    pub diagnostics: Vec<Diagnostic>,
    /// Unobserved coverage gaps in canonical order.
    pub gaps: Vec<CoverageGap>,
    /// Reconciliation counters.
    pub coverage: ReconcileCoverage,
}

/// Why reconciliation could not produce a ledger.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReconcileError {
    /// Deriving or registering an ID failed, including identity collisions.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// A request observation carried a key for another entity.
    #[error("request observation at {evidence:?} carries a {prefix} key")]
    WrongPrefix {
        /// The observation.
        evidence: EvidenceRef,
        /// The key's prefix.
        prefix: IdPrefix,
    },
    /// A normalized entity carries an identity with the wrong prefix.
    #[error("{entity} carries a {prefix} identity")]
    WrongEntityPrefix {
        /// The normalized entity kind.
        entity: &'static str,
        /// The identity's prefix.
        prefix: IdPrefix,
    },
    /// An observation without keys sits beyond the offsets a canonical key can carry.
    #[error("record offset {0:?} is too large for an artifact-local key")]
    OffsetOutOfRange(EvidenceRef),
    /// A selector returned an index outside the revisions it was given.
    #[error("revision selector {rule} chose index {selected} of {count} revisions")]
    InvalidRevisionChoice {
        /// The selector's rule name.
        rule: &'static str,
        /// The returned index.
        selected: usize,
        /// The number of revisions.
        count: usize,
    },
    /// More request observations arrived than one reconciliation holds.
    #[error(
        "{observations} request observations exceed the reconciliation capacity of {maximum} compact rows (2 GiB)"
    )]
    CapacityExceeded {
        /// The observations received.
        observations: usize,
        /// The most observations one reconciliation accepts.
        maximum: usize,
    },
}

/// The most request observations one reconciliation accepts: 2 GiB of observation rows.
///
/// This is a safety net, not a budget users tune. Whole history is far below it, and an
/// input above it fails with [`ReconcileError::CapacityExceeded`] before any request is
/// built rather than growing without bound.
pub const MAX_OBSERVATIONS: usize = 2 * 1024 * 1024 * 1024 / size_of::<RequestObservation>();

/// Refuses more than `maximum` observations.
fn ensure_capacity(observations: usize, maximum: usize) -> Result<(), ReconcileError> {
    if observations > maximum {
        return Err(ReconcileError::CapacityExceeded { observations, maximum });
    }
    Ok(())
}

/// A linked set's canonical ID, the basis of its key, and its other IDs in order.
struct LinkedRequest {
    id: AnalyticalId,
    basis: IdentityBasis,
    aliases: Box<[AnalyticalId]>,
}

/// A linked set whose observations disagree on a revision-invariant field, so each member
/// becomes its own request under its artifact-local key.
struct SplitSet {
    /// The set's first position in group order.
    start: usize,
    /// The fields the members disagree on.
    fields: BTreeSet<String>,
    /// Each member's artifact-local key, in canonical order.
    keys: Vec<DerivedKey>,
}

/// Reconciles normalized observations into a ledger; see the module documentation.
pub fn reconcile(
    input: ReconcileInput,
    selector: &dyn RevisionSelector,
) -> Result<Ledger, ReconcileError> {
    ensure_capacity(input.requests.len(), MAX_OBSERVATIONS)?;
    let ReconcileInput {
        threads,
        relationships,
        requests,
        tool_actions,
        limit_observations,
        links,
        mut gaps,
        mut diagnostics,
    } = input;
    let mut coverage =
        ReconcileCoverage { observations: count(requests.len()), ..ReconcileCoverage::default() };

    let mut registry = IdentityRegistry::new();
    let threads = reconcile_threads(threads, &mut registry, &mut diagnostics)?;
    let thread_ids = canonical_thread_ids(&threads);
    let relationships = reconcile_relationships(relationships, &thread_ids)?;
    let mut observations = canonicalize_request_owners(requests, &thread_ids);
    dedupe_rereads(&mut observations, &mut diagnostics, &mut coverage);

    // Register every key and move observations into linked sets: sets in order of their
    // lowest ID, members in canonical order, each set one contiguous run.
    let grouping = grouping::group(&mut observations, &links)?;
    drop(links);
    let mut splits = find_splits(&observations, &grouping, selector)?;
    let ranks = grouping.into_ranks();

    // Requests are built from the last set backwards, and each built set's observations are
    // truncated. Request rows are narrower than observation rows, so the request chunks
    // reuse the observation chunks freed before them, and the peak footprint holds about
    // one table rather than both. The build order does not change the ledger:
    // - Every request row is sorted by ID in `Requests::from_reversed`, which resolves a
    //   repeated ID as the forward build did, because parts are built last first within a
    //   set too, so the rows read backwards are in forward order.
    // - Diagnostics are sorted in `diagnostics::compact` before anything reads them, and
    //   coverage counters are sums.
    // - Candidate sets are the components of `candidates`, which do not depend on link order.
    // - Selectors are deterministic functions of one request's revisions, which keep
    //   canonical order.
    // - The errors a forward build would meet first are kept: `find_splits` registered
    //   every split key in forward order, and `first_choice_error` rescans forwards.
    let mut requests = ChunkedVec::new();
    let mut candidates = LinkGraph::new();
    let mut end = observations.len();
    while let Some(&rank) = end.checked_sub(1).and_then(|last| ranks.get(last)) {
        let start =
            ranks[..end].iter().rposition(|other| *other != rank).map_or(0, |before| before + 1);
        let split = if splits.last().is_some_and(|split| split.start == start) {
            splits.pop()
        } else {
            None
        };
        if split.is_some() {
            coverage.conflicting_keys = coverage.conflicting_keys.saturating_add(1);
        }
        let members: Vec<&RequestObservation> =
            (start..end).map(|index| &observations[index]).collect();
        let built =
            build_set(&members, split, selector, &mut requests, &mut candidates, &mut diagnostics);
        drop(members);
        if let Err(error) = built {
            return Err(first_choice_error(&observations, &ranks, end, selector).unwrap_or(error));
        }
        observations.truncate(start);
        end = start;
    }
    drop(observations);
    drop(ranks);
    let mut requests = Requests::from_reversed(requests);
    let candidate_sets = resolve_candidate_sets(&candidates, &mut requests, &mut diagnostics);
    coverage.candidate_sets = count(candidate_sets.len());
    coverage.requests = count(requests.len());
    coverage.copy_only_requests =
        count(requests.values().filter(|r| r.counting == Counting::CopyOnly).count());
    coverage.unresolved_requests = count(
        requests.values().filter(|r| matches!(r.counting, Counting::Unresolved { .. })).count(),
    );
    coverage.requests_without_usage =
        count(requests.values().filter(|r| r.usage.is_none()).count());
    let request_ids = canonical_request_ids(&requests);
    let tool_actions =
        reconcile_tool_actions(tool_actions, &request_ids, &mut registry, &mut diagnostics)?;
    let limit_observations =
        reconcile_limit_observations(limit_observations, &thread_ids, &request_ids)?;
    let diagnostics = super::diagnostics::compact(diagnostics);
    gaps.sort();
    gaps.dedup();
    Ok(Ledger {
        threads,
        relationships,
        requests,
        tool_actions,
        limit_observations,
        candidate_sets,
        revision_rule: selector.rule(),
        diagnostics,
        gaps,
        coverage,
    })
}

fn reconcile_threads(
    mut observations: Vec<Thread>,
    registry: &mut IdentityRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<BTreeMap<AnalyticalId, Thread>, ReconcileError> {
    observations.sort();
    observations.dedup();
    let mut graph = LinkGraph::new();
    for thread in &observations {
        register_entity_identity(registry, &thread.identity, IdPrefix::Thread, "thread")?;
        graph.insert(&thread.identity.id);
        for alias in &thread.aliases {
            register_entity_identity(registry, alias, IdPrefix::Thread, "thread alias")?;
            graph.link(&thread.identity.id, &alias.id);
        }
    }
    let mut groups: BTreeMap<AnalyticalId, Vec<Thread>> = BTreeMap::new();
    for thread in observations {
        groups.entry(graph.find(&thread.identity.id)).or_default().push(thread);
    }
    let mut reconciled = BTreeMap::new();
    for group in groups.into_values() {
        let canonical = group
            .iter()
            .min_by_key(|thread| (thread.basis, &thread.identity.id))
            .expect("thread group is non-empty");
        let identity = canonical.identity.clone();
        let basis = canonical.basis;
        let aliases = group
            .iter()
            .flat_map(|thread| std::iter::once(&thread.identity).chain(thread.aliases.iter()))
            .filter(|candidate| candidate.id != identity.id)
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let evidence = merged_evidence(group.iter().flat_map(|thread| &thread.evidence));
        let mut conflicts = BTreeSet::new();
        let native_key =
            merge_native_keys(group.iter().map(|thread| &thread.native_key), &mut conflicts);
        let source =
            merge_basis("source", group.iter().map(|thread| &thread.source), &mut conflicts);
        let initiator =
            merge_basis("initiator", group.iter().map(|thread| &thread.initiator), &mut conflicts);
        let purpose =
            merge_basis("purpose", group.iter().map(|thread| &thread.purpose), &mut conflicts);
        let execution_environment = merge_basis(
            "execution_environment",
            group.iter().map(|thread| &thread.execution_environment),
            &mut conflicts,
        );
        let project =
            merge_basis("project", group.iter().map(|thread| &thread.project), &mut conflicts);
        let account =
            merge_basis("account", group.iter().map(|thread| &thread.account), &mut conflicts);
        diagnose_entity_conflicts("thread", &identity.id, &evidence, &conflicts, diagnostics);
        reconciled.insert(
            identity.id.clone(),
            Thread {
                identity,
                basis,
                aliases,
                native_key,
                source,
                initiator,
                purpose,
                execution_environment,
                project,
                account,
                evidence,
            },
        );
    }
    Ok(reconciled)
}

fn reconcile_relationships(
    observations: Vec<Relationship>,
    thread_ids: &BTreeMap<AnalyticalId, AnalyticalId>,
) -> Result<Vec<Relationship>, ReconcileError> {
    let mut groups: BTreeMap<(RelationshipKind, AnalyticalId, AnalyticalId), Vec<Relationship>> =
        BTreeMap::new();
    for mut relationship in observations {
        for endpoint in [&relationship.from, &relationship.to] {
            if endpoint.prefix() != IdPrefix::Thread {
                return Err(ReconcileError::WrongEntityPrefix {
                    entity: "relationship endpoint",
                    prefix: endpoint.prefix(),
                });
            }
        }
        relationship.from = canonical_id(&relationship.from, thread_ids);
        relationship.to = canonical_id(&relationship.to, thread_ids);
        groups
            .entry((relationship.kind.clone(), relationship.from.clone(), relationship.to.clone()))
            .or_default()
            .push(relationship);
    }
    Ok(groups
        .into_iter()
        .map(|((kind, from, to), group)| Relationship {
            kind,
            from,
            to,
            confidence: if group.iter().any(|item| item.confidence == Confidence::Proven) {
                Confidence::Proven
            } else {
                Confidence::Inferred
            },
            evidence: merged_evidence(group.iter().flat_map(|item| &item.evidence)),
        })
        .collect())
}

fn reconcile_tool_actions(
    mut observations: Vec<ToolAction>,
    request_ids: &BTreeMap<AnalyticalId, AnalyticalId>,
    registry: &mut IdentityRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<BTreeMap<AnalyticalId, ToolAction>, ReconcileError> {
    observations.sort();
    observations.dedup();
    let mut groups: BTreeMap<AnalyticalId, Vec<ToolAction>> = BTreeMap::new();
    for mut action in observations {
        register_entity_identity(registry, &action.identity, IdPrefix::Action, "tool action")?;
        if let Some(request) = &action.request {
            if request.prefix() != IdPrefix::Request {
                return Err(ReconcileError::WrongEntityPrefix {
                    entity: "tool action request",
                    prefix: request.prefix(),
                });
            }
            action.request = Some(canonical_id(request, request_ids));
        }
        groups.entry(action.identity.id.clone()).or_default().push(action);
    }
    let mut reconciled = BTreeMap::new();
    for (id, group) in groups {
        let first = group.first().expect("tool action group is non-empty");
        let evidence = merged_evidence(group.iter().flat_map(|action| &action.evidence));
        let mut conflicts = BTreeSet::new();
        let call_id =
            merge_optional("call_id", group.iter().map(|action| &action.call_id), &mut conflicts);
        let tool_name = merge_optional(
            "tool_name",
            group.iter().map(|action| &action.tool_name),
            &mut conflicts,
        );
        let request =
            merge_optional("request", group.iter().map(|action| &action.request), &mut conflicts);
        diagnose_entity_conflicts("tool action", &id, &evidence, &conflicts, diagnostics);
        reconciled.insert(
            id,
            ToolAction {
                identity: first.identity.clone(),
                basis: group.iter().map(|action| action.basis).min().unwrap_or(first.basis),
                call_id,
                tool_name,
                request,
                evidence,
            },
        );
    }
    Ok(reconciled)
}

fn reconcile_limit_observations(
    mut observations: Vec<ProviderLimitObservation>,
    thread_ids: &BTreeMap<AnalyticalId, AnalyticalId>,
    request_ids: &BTreeMap<AnalyticalId, AnalyticalId>,
) -> Result<Vec<ProviderLimitObservation>, ReconcileError> {
    for observation in &mut observations {
        for (entity, owner, prefix) in [
            ("provider limit thread", observation.owner_thread.as_ref(), IdPrefix::Thread),
            ("provider limit request", observation.owner_request.as_ref(), IdPrefix::Request),
        ] {
            if let Some(owner) = owner {
                if owner.prefix() != prefix {
                    return Err(ReconcileError::WrongEntityPrefix {
                        entity,
                        prefix: owner.prefix(),
                    });
                }
            }
        }
        observation.owner_thread =
            observation.owner_thread.as_ref().map(|owner| canonical_id(owner, thread_ids));
        observation.owner_request =
            observation.owner_request.as_ref().map(|owner| canonical_id(owner, request_ids));
    }
    observations.sort_by_cached_key(limit_sort_key);
    observations.dedup();
    let mut reconciled: Vec<ProviderLimitObservation> = Vec::with_capacity(observations.len());
    let mut previous: BTreeMap<LimitStreamKey, Box<str>> = BTreeMap::new();
    for observation in observations {
        let stream = limit_stream_key(&observation);
        let signature = observation.native.clone();
        let repeated = previous.get(&stream) == Some(&signature);
        if !repeated {
            reconciled.push(observation);
        }
        previous.insert(stream, signature);
    }
    Ok(reconciled)
}

fn canonical_thread_ids(
    threads: &BTreeMap<AnalyticalId, Thread>,
) -> BTreeMap<AnalyticalId, AnalyticalId> {
    threads
        .iter()
        .flat_map(|(id, thread)| {
            std::iter::once((id.clone(), id.clone()))
                .chain(thread.aliases.iter().map(|alias| (alias.id.clone(), id.clone())))
        })
        .collect()
}

/// Maps each request alias to its canonical ID. A canonical ID maps to itself through
/// [`canonical_id`]'s fallback, so only aliases need entries.
fn canonical_request_ids(requests: &Requests) -> BTreeMap<AnalyticalId, AnalyticalId> {
    requests
        .iter()
        .flat_map(|(id, request)| request.aliases.iter().map(|alias| (alias.clone(), id.clone())))
        .collect()
}

fn canonical_id(
    id: &AnalyticalId,
    canonical: &BTreeMap<AnalyticalId, AnalyticalId>,
) -> AnalyticalId {
    canonical.get(id).cloned().unwrap_or_else(|| id.clone())
}

fn canonicalize_request_owners(
    mut observations: ChunkedVec<RequestObservation>,
    thread_ids: &BTreeMap<AnalyticalId, AnalyticalId>,
) -> ChunkedVec<RequestObservation> {
    for observation in &mut observations {
        observation.owner = match &observation.owner {
            OwnerEvidence::Proven(thread) => {
                OwnerEvidence::Proven(canonical_id(thread, thread_ids))
            }
            OwnerEvidence::None => OwnerEvidence::None,
        };
    }
    observations
}

fn register_entity_identity(
    registry: &mut IdentityRegistry,
    identity: &super::identity::StoredIdentity,
    prefix: IdPrefix,
    entity: &'static str,
) -> Result<(), ReconcileError> {
    if identity.id.prefix() != prefix || identity.key.prefix != prefix {
        let actual =
            if identity.id.prefix() == prefix { identity.key.prefix } else { identity.id.prefix() };
        return Err(ReconcileError::WrongEntityPrefix { entity, prefix: actual });
    }
    registry.register_stored(identity)?;
    Ok(())
}

fn merged_evidence<'a>(evidence: impl IntoIterator<Item = &'a EvidenceRef>) -> Vec<EvidenceRef> {
    evidence.into_iter().cloned().collect::<BTreeSet<_>>().into_iter().collect()
}

fn merge_native_keys<'a>(
    maps: impl IntoIterator<Item = &'a BTreeMap<String, String>>,
    conflicts: &mut BTreeSet<String>,
) -> BTreeMap<String, String> {
    let mut values: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for map in maps {
        for (field, value) in map {
            values.entry(field).or_default().insert(value);
        }
    }
    values
        .into_iter()
        .filter_map(|(field, values)| {
            if values.len() == 1 {
                Some((field.to_owned(), values.into_iter().next().unwrap_or_default().to_owned()))
            } else {
                conflicts.insert(format!("native_key.{field}"));
                None
            }
        })
        .collect()
}

fn merge_basis<'a>(
    field: &str,
    values: impl IntoIterator<Item = &'a Basis<String>>,
    conflicts: &mut BTreeSet<String>,
) -> Basis<String> {
    let known: Vec<&Basis<String>> =
        values.into_iter().filter(|value| value.value().is_some()).collect();
    let distinct: BTreeSet<&str> =
        known.iter().filter_map(|value| value.value().map(String::as_str)).collect();
    if distinct.len() > 1 {
        conflicts.insert(field.to_owned());
        Basis::Unknown
    } else {
        known.into_iter().min().cloned().unwrap_or(Basis::Unknown)
    }
}

fn merge_optional<'a, T: Clone + Ord + 'a>(
    field: &str,
    values: impl IntoIterator<Item = &'a Option<T>>,
    conflicts: &mut BTreeSet<String>,
) -> Option<T> {
    let distinct: BTreeSet<&T> = values.into_iter().filter_map(Option::as_ref).collect();
    if distinct.len() > 1 {
        conflicts.insert(field.to_owned());
        None
    } else {
        distinct.into_iter().next().cloned()
    }
}

fn diagnose_entity_conflicts(
    entity: &str,
    id: &AnalyticalId,
    evidence: &[EvidenceRef],
    conflicts: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !conflicts.is_empty() {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::ConflictingSharedKey,
            Some(id.clone()),
            evidence.iter().cloned(),
            format!("{entity} observations disagree on {}", join(conflicts.iter())),
        ));
    }
}

type LimitStreamKey =
    (AnalyticalId, Option<AnalyticalId>, Option<AnalyticalId>, Option<String>, Option<String>);

type LimitSortKey = (
    EvidenceRef,
    Option<AnalyticalId>,
    Option<AnalyticalId>,
    Option<String>,
    Option<String>,
    Basis<Timestamp>,
    Box<str>,
);

fn limit_stream_key(observation: &ProviderLimitObservation) -> LimitStreamKey {
    (
        observation.evidence.source.clone(),
        observation.owner_thread.clone(),
        observation.owner_request.clone(),
        observation.limit_name.clone(),
        observation.window.clone(),
    )
}

fn limit_sort_key(observation: &ProviderLimitObservation) -> LimitSortKey {
    (
        observation.evidence.clone(),
        observation.owner_thread.clone(),
        observation.owner_request.clone(),
        observation.limit_name.clone(),
        observation.window.clone(),
        observation.observed_at.clone(),
        observation.native.clone(),
    )
}

/// Sorts observations into canonical order and removes re-reads in place: identical
/// observations count once, and a later observation of a kept record location with
/// different content is diagnosed and dropped.
fn dedupe_rereads(
    observations: &mut ChunkedVec<RequestObservation>,
    diagnostics: &mut Vec<Diagnostic>,
    coverage: &mut ReconcileCoverage,
) {
    // Observations that compare equal are identical, so an unstable sort gives the stable
    // sort's result; it sorts a permutation and moves rows only by swaps.
    observations.sort_unstable();
    let before = observations.len();
    observations.dedup();
    coverage.rereads = count(before.saturating_sub(observations.len()));
    let mut previous: Option<EvidenceRef> = None;
    observations.retain(|observation| {
        if previous.as_ref() == Some(&observation.evidence) {
            coverage.conflicting_rereads = coverage.conflicting_rereads.saturating_add(1);
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ConflictingReread,
                None,
                [observation.evidence.clone()],
                "one record location was observed with different content",
            ));
            return false;
        }
        if observation.role == ObservationRole::Copy {
            coverage.copies = coverage.copies.saturating_add(1);
        }
        previous = Some(observation.evidence.clone());
        true
    });
}

fn artifact_local(evidence: &EvidenceRef) -> Result<DerivedKey, ReconcileError> {
    Ok(artifact_local_key(IdPrefix::Request, &evidence.source, evidence.offset)
        .ok_or_else(|| ReconcileError::OffsetOutOfRange(evidence.clone()))?
        .derive()?)
}

/// Finds every split set in group order, and derives and registers its members'
/// artifact-local keys as a forward build would, just before building each member.
///
/// A split key's ID is checked against the observation keys and then against earlier split
/// keys. The first failure is returned unless a revision choice before it fails first.
fn find_splits(
    observations: &ChunkedVec<RequestObservation>,
    grouping: &Grouping,
    selector: &dyn RevisionSelector,
) -> Result<Vec<SplitSet>, ReconcileError> {
    let mut splits = Vec::new();
    let mut split_checks: BTreeMap<AnalyticalId, [u8; 8]> = BTreeMap::new();
    let mut start = 0;
    for run in grouping.ranks.chunk_by(|left, right| left == right) {
        let end = start + run.len();
        let fields = conflicting_fields((start..end).map(|index| &observations[index]));
        if !fields.is_empty() {
            let mut keys = Vec::with_capacity(run.len());
            for position in start..end {
                let registered = artifact_local(&observations[position].evidence).and_then(|key| {
                    let registered = grouping
                        .registered_check(&key.id)
                        .or_else(|| split_checks.get(&key.id).copied());
                    match registered {
                        Some(check) if check != key.check => {
                            Err(IdentityError::DigestCollision { id: key.id }.into())
                        }
                        Some(_) => Ok(key),
                        None => {
                            split_checks.insert(key.id.clone(), key.check);
                            Ok(key)
                        }
                    }
                });
                match registered {
                    Ok(key) => keys.push(key),
                    Err(error) => {
                        let earlier =
                            first_choice_error(observations, &grouping.ranks, position, selector);
                        return Err(earlier.unwrap_or(error));
                    }
                }
            }
            splits.push(SplitSet { start, fields, keys });
        }
        start = end;
    }
    Ok(splits)
}

/// The first invalid revision choice, in forward build order, among the requests whose
/// observations all lie before position `until`: sets in group order, and a split set's
/// parts in canonical order.
fn first_choice_error(
    observations: &ChunkedVec<RequestObservation>,
    ranks: &[u32],
    until: usize,
    selector: &dyn RevisionSelector,
) -> Option<ReconcileError> {
    let mut start = 0;
    for run in ranks.chunk_by(|left, right| left == right) {
        if start >= until {
            break;
        }
        let end = start + run.len();
        let members: Vec<&RequestObservation> =
            (start..end).map(|index| &observations[index]).collect();
        let split = !conflicting_fields(members.iter().copied()).is_empty();
        let part_size = if split { 1 } else { members.len() };
        for (part_index, part) in members.chunks(part_size).enumerate() {
            if start + (part_index + 1) * part_size > until {
                break;
            }
            let revisions: Vec<&RequestObservation> = part
                .iter()
                .copied()
                .filter(|o| o.role == ObservationRole::Original && o.usage.is_some())
                .collect();
            if !revisions.is_empty() {
                if let Err(error) = choose_revision(&revisions, selector) {
                    return Some(error);
                }
            }
        }
        start = end;
    }
    None
}

/// Builds one linked set's requests: one request, or one per member of a split set, which
/// then form a candidate set.
fn build_set(
    members: &[&RequestObservation],
    split: Option<SplitSet>,
    selector: &dyn RevisionSelector,
    requests: &mut ChunkedVec<Request>,
    candidates: &mut LinkGraph,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ReconcileError> {
    let Some(split) = split else {
        if let Some(request) = build_request(members, None, selector, diagnostics)? {
            requests.push(request);
        }
        return Ok(());
    };
    // Parts are built last first, like sets, so the request rows read backwards are in
    // forward build order.
    let mut split_ids = Vec::with_capacity(members.len());
    for (member, key) in members.iter().zip(&split.keys).rev() {
        let Some(request) = build_request(&[*member], Some(key), selector, diagnostics)? else {
            continue;
        };
        split_ids.push(request.id().clone());
        requests.push(request);
    }
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::ConflictingSharedKey,
        None,
        members.iter().map(|member| member.evidence.clone()),
        format!("observations sharing a key disagree on {}", join(split.fields.iter())),
    ));
    for pair in split_ids.windows(2) {
        candidates.link(&pair[0], &pair[1]);
    }
    Ok(())
}

fn index_u32(index: usize) -> u32 {
    u32::try_from(index).expect("fewer than 2^32 request observations and keys")
}

/// The highest-precedence key's ID, then the lowest ID; every other distinct ID is an
/// alias.
fn resolve_compact_set<'a>(
    members: impl IntoIterator<Item = &'a DerivedKey>,
) -> Option<LinkedRequest> {
    let unique: BTreeSet<(u8, IdentityBasis, &AnalyticalId)> =
        members.into_iter().map(|key| (key.precedence, key.basis, &key.id)).collect();
    let mut ranked = unique.into_iter();
    let (_, basis, canonical) = ranked.next()?;
    let mut aliases: Vec<AnalyticalId> =
        ranked.filter(|(_, _, id)| *id != canonical).map(|(_, _, id)| id.clone()).collect();
    aliases.sort();
    aliases.dedup();
    Some(LinkedRequest { id: canonical.clone(), basis, aliases: aliases.into_boxed_slice() })
}

/// Revision-invariant fields on which the set's observations disagree.
fn conflicting_fields<'a>(
    members: impl Iterator<Item = &'a RequestObservation> + Clone,
) -> BTreeSet<String> {
    // Almost every set has at most one member with an invariant, or members that carry one
    // identical invariant, and those cannot disagree.
    let mut carried =
        members.clone().map(|member| &member.invariants).filter(|list| !list.is_empty());
    match carried.next() {
        None => return BTreeSet::new(),
        Some(first) if first.len() == 1 && carried.all(|list| list == first) => {
            return BTreeSet::new();
        }
        Some(_) => {}
    }
    let mut values: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for member in members {
        for (field, value) in &member.invariants {
            values.entry(field).or_default().insert(value.as_str());
        }
    }
    values
        .into_iter()
        .filter(|(_, distinct)| distinct.len() > 1)
        .map(|(field, _)| field.to_owned())
        .collect()
}

/// The selector's choice among non-empty revisions, refused when it names none of them.
fn choose_revision(
    revisions: &[&RequestObservation],
    selector: &dyn RevisionSelector,
) -> Result<RevisionChoice, ReconcileError> {
    let choice = selector.select(revisions);
    if choice.selected >= revisions.len() {
        return Err(ReconcileError::InvalidRevisionChoice {
            rule: selector.rule(),
            selected: choice.selected,
            count: revisions.len(),
        });
    }
    Ok(choice)
}

/// Builds one request from its observations in canonical order. A split set's member is
/// built alone under its artifact-local `local_key`.
fn build_request(
    observations: &[&RequestObservation],
    local_key: Option<&DerivedKey>,
    selector: &dyn RevisionSelector,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Option<Request>, ReconcileError> {
    // A split part has only its artifact-local ID; otherwise every key of every member
    // counts, and a member without keys already carries its artifact-local key.
    let linked = match local_key {
        Some(key) => resolve_compact_set([key]),
        None => {
            resolve_compact_set(observations.iter().flat_map(|observation| observation.keys.iter()))
        }
    };
    let Some(linked) = linked else {
        return Ok(None);
    };
    let id = linked.id.clone();
    let originals: Vec<&RequestObservation> =
        observations.iter().copied().filter(|o| o.role == ObservationRole::Original).collect();
    let revisions: Vec<&RequestObservation> =
        originals.iter().copied().filter(|o| o.usage.is_some()).collect();

    let (usage, selected) = if revisions.is_empty() {
        (None, None)
    } else {
        let choice = choose_revision(&revisions, selector)?;
        let chosen = revisions[choice.selected];
        if !choice.disagreements.is_empty() {
            let mut disagreements = choice.disagreements.clone();
            disagreements.sort();
            disagreements.dedup();
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::RevisionDisagreement,
                Some(id.clone()),
                revisions.iter().map(|r| r.evidence.clone()),
                format!("{}: {}", selector.rule(), join(disagreements.iter())),
            ));
        }
        // Revisions are originals, whose evidence the request keeps in the same order.
        let position = originals
            .iter()
            .position(|original| original.evidence == chosen.evidence)
            .map(index_u32);
        let usage = chosen.usage.zip(position).map(|(usage, evidence)| SelectedUsage {
            revision: UsageRevision { usage, model_usage: chosen.model_usage.clone() },
            evidence,
            status: choice.status,
        });
        let selected = usage.as_ref().map(|_| chosen);
        (usage, selected)
    };

    let counting = if originals.is_empty() {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::CopyWithoutOriginal,
            Some(id.clone()),
            observations.iter().map(|o| o.evidence.clone()),
            "request observed only as copies; its usage is not counted",
        ));
        Counting::CopyOnly
    } else {
        Counting::Counted
    };

    Ok(Some(Request {
        basis: linked.basis,
        aliases: linked.aliases,
        ownership: ownership(observations, &id, diagnostics),
        first_seen: originals.iter().filter_map(|o| o.timestamp).min(),
        last_seen: originals.iter().filter_map(|o| o.timestamp).max(),
        model: model(&originals, selected, &id, diagnostics),
        effort: selected
            .and_then(|s| s.effort)
            .or_else(|| originals.iter().filter_map(|o| o.effort).min()),
        originals: index_u32(originals.len()),
        records: originals
            .iter()
            .chain(observations.iter().filter(|o| o.role == ObservationRole::Copy))
            .map(|o| o.evidence.clone())
            .collect(),
        usage,
        counting,
        id: linked.id,
    }))
}

fn ownership(
    observations: &[&RequestObservation],
    id: &AnalyticalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Ownership {
    let mut proven = BTreeSet::new();
    for observation in observations {
        if let OwnerEvidence::Proven(thread) = &observation.owner {
            proven.insert(thread.clone());
        }
    }
    if proven.len() > 1 {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::ConflictingOwners,
            Some(id.clone()),
            observations
                .iter()
                .filter(|o| matches!(o.owner, OwnerEvidence::Proven(_)))
                .map(|o| o.evidence.clone()),
            format!("proven owners {}", join(proven.iter())),
        ));
        return Ownership::Ambiguous { candidates: proven };
    }
    proven.into_iter().next().map_or(Ownership::Unknown, |thread| Ownership::Owned { thread })
}

fn model(
    originals: &[&RequestObservation],
    selected: Option<&RequestObservation>,
    id: &AnalyticalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ModelName> {
    let served: BTreeSet<&str> = originals
        .iter()
        .filter_map(|o| o.model.as_ref())
        .filter(|m| m.basis == ModelBasis::Served)
        .map(|m| m.name.as_str())
        .collect();
    if served.len() > 1 {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::ConflictingModels,
            Some(id.clone()),
            originals.iter().filter(|o| o.model.is_some()).map(|o| o.evidence.clone()),
            format!("served models {}", join(served.iter())),
        ));
    }
    selected.and_then(|s| s.model).or_else(|| {
        originals
            .iter()
            .filter_map(|o| o.model)
            .min_by(|a, b| (a.basis, a.name).cmp(&(b.basis, b.name)))
    })
}

fn resolve_candidate_sets(
    candidates: &LinkGraph,
    requests: &mut Requests,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<BTreeSet<AnalyticalId>> {
    let mut sets = Vec::new();
    for members in candidates.components().into_values() {
        if members.len() < 2 {
            continue;
        }
        let counted = members
            .iter()
            .filter_map(|id| requests.get(id))
            .filter(|request| request.counting == Counting::Counted)
            .map(|request| (request.basis, request.id().clone()))
            .min();
        if let Some((_, winner)) = counted {
            let mut evidence = Vec::new();
            for id in &members {
                if let Some(request) = requests.get_mut(id) {
                    if *id != winner && request.counting == Counting::Counted {
                        request.counting = Counting::Unresolved { counted: winner.clone() };
                        evidence.extend(request.evidence().iter().cloned());
                    }
                }
            }
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::UnresolvedCandidate,
                Some(winner.clone()),
                evidence,
                format!("candidate set of {} requests counts one", members.len()),
            ));
        }
        sets.push(members);
    }
    sets
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

fn join<T: std::fmt::Display>(items: impl IntoIterator<Item = T>) -> String {
    items.into_iter().map(|item| item.to_string()).collect::<Vec<_>>().join(", ")
}

mod grouping;

#[cfg(test)]
mod tests;
