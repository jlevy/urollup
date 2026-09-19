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
//! 2. **Identity:** every key's ID is derived and registered, so two keys yielding one ID
//!    fail with [`ReconcileError::Identity`]. An observation without a usable key gets its
//!    artifact-local ID. Observations sharing any key ID, or joined by a [`LineageLink`],
//!    form one linked set.
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

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use jiff::Timestamp;

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
use crate::sources::evidence::{EvidenceRef, SourceTable};

/// Whether an observation is the request's own record or a copy of it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObservationRole {
    /// The request's own record in its owner's source.
    Original,
    /// A replay nested in or copied into another record or file; never counted.
    Copy,
}

/// A native revision sequence stored as `n + 1` so `Option` stays 8 bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NativeSequence(NonZeroU64);

impl NativeSequence {
    /// The sequence, or `None` if `value + 1` does not fit in `u64`.
    pub fn new(value: u64) -> Option<Self> {
        value.checked_add(1).and_then(NonZeroU64::new).map(Self)
    }

    /// The recorded sequence.
    pub fn get(self) -> u64 {
        self.0.get().saturating_sub(1)
    }
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
    pub sequence: Option<NativeSequence>,
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

const _: () = assert!(std::mem::size_of::<RequestObservation>() <= 224);

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
            revisions.iter().map(|revision| revision.sequence.map(NativeSequence::get)).collect();
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
    /// Request observations, in any order.
    pub requests: Vec<RequestObservation>,
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
    /// `src-` IDs in the index order [`EvidenceRef::source`] uses.
    pub source_table: SourceTable,
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
    /// `src-` IDs in the index order [`EvidenceRef::source`] uses.
    pub source_table: SourceTable,
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

/// Request key IDs as union-find nodes, with the further digest bits that catch two
/// different keys deriving one ID without storing either key.
///
/// Nodes are indices, so linking hundreds of thousands of observations allocates a few
/// flat vectors rather than tree nodes per ID. Each ID is stored once in `ids`; lookup
/// is an open-addressed table of those indices. A set's root is always its lowest ID, so
/// roots do not depend on link order.
#[derive(Clone, Debug, Eq, PartialEq)]
struct KeyGraph {
    ids: Vec<AnalyticalId>,
    checks: Vec<Option<[u8; 8]>>,
    parent: Vec<u32>,
    /// Open-addressed node indices. `u32::MAX` is empty. Length is 0 or a power of two.
    slots: Vec<u32>,
}

const EMPTY_SLOT: u32 = u32::MAX;

impl KeyGraph {
    /// An empty graph, or one sized for about `nodes` IDs without growing the ID vectors.
    fn with_capacity(nodes: usize) -> Self {
        let slots = if nodes == 0 { 0 } else { slot_len(nodes) };
        Self {
            ids: Vec::with_capacity(nodes),
            checks: Vec::with_capacity(nodes),
            parent: Vec::with_capacity(nodes),
            slots: vec![EMPTY_SLOT; slots],
        }
    }

    /// The node for `id`, added unregistered and alone when new.
    fn node(&mut self, id: &AnalyticalId) -> u32 {
        if let Some(node) = self.lookup(id) {
            return node;
        }
        let node = index_u32(self.ids.len());
        self.ids.push(id.clone());
        self.checks.push(None);
        self.parent.push(node);
        self.insert_slot(id, node);
        node
    }

    fn lookup(&self, id: &AnalyticalId) -> Option<u32> {
        if self.slots.is_empty() {
            return None;
        }
        let mask = self.slots.len() - 1;
        let mut slot = probe_slot(id, mask);
        loop {
            let node = self.slots[slot];
            if node == EMPTY_SLOT {
                return None;
            }
            if self.ids[node as usize] == *id {
                return Some(node);
            }
            slot = (slot + 1) & mask;
        }
    }

    fn insert_slot(&mut self, id: &AnalyticalId, node: u32) {
        if self.ids.len().saturating_mul(2) > self.slots.len() {
            self.rehash();
            return;
        }
        place(&mut self.slots, id, node);
    }

    fn rehash(&mut self) {
        let mut slots = vec![EMPTY_SLOT; slot_len(self.ids.len())];
        for (index, id) in self.ids.iter().enumerate() {
            place(&mut slots, id, index_u32(index));
        }
        self.slots = slots;
    }

    /// Registers a derived key's check bits, failing when another key derived its ID.
    fn register(&mut self, key: &DerivedKey) -> Result<u32, IdentityError> {
        let node = self.node(&key.id);
        match &mut self.checks[node as usize] {
            Some(check) if *check != key.check => {
                Err(IdentityError::DigestCollision { id: key.id.clone() })
            }
            Some(_) => Ok(node),
            slot @ None => {
                *slot = Some(key.check);
                Ok(node)
            }
        }
    }

    fn id(&self, node: u32) -> &AnalyticalId {
        &self.ids[node as usize]
    }

    fn find(&mut self, node: u32) -> u32 {
        let mut root = node;
        while self.parent[root as usize] != root {
            root = self.parent[root as usize];
        }
        let mut current = node;
        while current != root {
            let next = self.parent[current as usize];
            self.parent[current as usize] = root;
            current = next;
        }
        root
    }

    fn link(&mut self, a: u32, b: u32) {
        let (root_a, root_b) = (self.find(a), self.find(b));
        match self.id(root_a).cmp(self.id(root_b)) {
            std::cmp::Ordering::Less => self.parent[root_b as usize] = root_a,
            std::cmp::Ordering::Greater => self.parent[root_a as usize] = root_b,
            std::cmp::Ordering::Equal => {}
        }
    }
}

impl Default for KeyGraph {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

fn slot_len(nodes: usize) -> usize {
    nodes.saturating_mul(2).max(8).next_power_of_two()
}

fn probe_slot(id: &AnalyticalId, mask: usize) -> usize {
    let hash = id.table_hash() & u64::try_from(mask).expect("slot mask fits u64");
    usize::try_from(hash).expect("masked hash fits usize")
}

fn place(slots: &mut [u32], id: &AnalyticalId, node: u32) {
    let mask = slots.len() - 1;
    let mut slot = probe_slot(id, mask);
    loop {
        if slots[slot] == EMPTY_SLOT {
            slots[slot] = node;
            return;
        }
        slot = (slot + 1) & mask;
    }
}

/// A linked set's canonical ID, the basis of its key, and its other IDs in order.
struct LinkedRequest {
    id: AnalyticalId,
    basis: IdentityBasis,
    aliases: Box<[AnalyticalId]>,
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
        mut links,
        mut gaps,
        mut diagnostics,
        source_table,
    } = input;
    let mut coverage =
        ReconcileCoverage { observations: count(requests.len()), ..ReconcileCoverage::default() };

    let mut registry = IdentityRegistry::new();
    let threads = reconcile_threads(threads, &mut registry, &mut diagnostics)?;
    let thread_ids = canonical_thread_ids(&threads);
    let relationships = reconcile_relationships(relationships, &thread_ids)?;
    let mut observations = canonicalize_request_owners(requests, &thread_ids);
    dedupe_rereads(&mut observations, &mut diagnostics, &mut coverage);

    // Register every key, linking keys that share an observation, then lineage links.
    let mut graph = KeyGraph::with_capacity(observations.len());
    let mut first_keys = Vec::with_capacity(observations.len());
    for observation in &mut observations {
        first_keys.push(resolve_identities(observation, &mut graph, &source_table)?);
    }
    links.sort();
    for link in &links {
        let (a, b) = (graph.node(&link.a), graph.node(&link.b));
        graph.link(a, b);
    }

    // Group observations by linked set: sets in order of their lowest ID, members in
    // canonical order.
    let mut order: Vec<(u32, u32)> = Vec::with_capacity(observations.len());
    for (index, first) in first_keys.into_iter().enumerate() {
        order.push((graph.find(first), index_u32(index)));
    }
    order.sort_unstable_by(|(left_root, left), (right_root, right)| {
        graph.id(*left_root).cmp(graph.id(*right_root)).then(left.cmp(right))
    });

    // One request per set, plus rare extra parts from conflicting keys; sizing up front
    // avoids doubling the widest vector of the run.
    let sets = order.windows(2).filter(|pair| pair[0].0 != pair[1].0).count()
        + usize::from(!order.is_empty());
    let mut requests = Vec::with_capacity(sets + sets / 32);
    let mut candidates = LinkGraph::new();
    let mut start = 0;
    while let Some(&(root, _)) = order.get(start) {
        let end = order[start..]
            .iter()
            .position(|(other, _)| *other != root)
            .map_or(order.len(), |offset| start + offset);
        let group = &order[start..end];
        {
            let members: Vec<&RequestObservation> =
                group.iter().map(|(_, index)| &observations[*index as usize]).collect();
            let split = conflicting_fields(&members);
            let split_evidence: Vec<EvidenceRef> = if split.is_empty() {
                Vec::new()
            } else {
                coverage.conflicting_keys = coverage.conflicting_keys.saturating_add(1);
                members.iter().map(|member| member.evidence).collect()
            };
            let parts: Vec<Vec<&RequestObservation>> = if split.is_empty() {
                vec![members]
            } else {
                members.into_iter().map(|member| vec![member]).collect()
            };
            let mut split_ids = Vec::new();
            for part in parts {
                let Some(request) = build_request(
                    &part,
                    !split.is_empty(),
                    selector,
                    &mut graph,
                    &mut diagnostics,
                    &source_table,
                )?
                else {
                    continue;
                };
                split_ids.push(request.id().clone());
                requests.push(request);
            }
            if !split.is_empty() {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::ConflictingSharedKey,
                    None,
                    split_evidence,
                    format!("observations sharing a key disagree on {}", join(split.iter())),
                ));
                for pair in split_ids.windows(2) {
                    candidates.link(&pair[0], &pair[1]);
                }
            }
        }
        // The group's request is built; free what its observations own before the next.
        for (_, index) in group {
            if let Some(observation) = observations.get_mut(*index as usize) {
                release_payload(observation);
            }
        }
        start = end;
    }
    drop(order);
    drop(observations);
    drop(graph);

    let mut requests = Requests::from_unsorted(requests);
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
        source_table,
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
    let mut previous: BTreeMap<LimitStreamKey, Name> = BTreeMap::new();
    for observation in observations {
        let stream = limit_stream_key(&observation);
        let signature = observation.native;
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
    mut observations: Vec<RequestObservation>,
    thread_ids: &BTreeMap<AnalyticalId, AnalyticalId>,
) -> Vec<RequestObservation> {
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
    evidence.into_iter().copied().collect::<BTreeSet<_>>().into_iter().collect()
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
            evidence.iter().copied(),
            format!("{entity} observations disagree on {}", join(conflicts.iter())),
        ));
    }
}

type LimitStreamKey = (u32, Option<AnalyticalId>, Option<AnalyticalId>, Option<Name>, Option<Name>);

type LimitSortKey = (
    EvidenceRef,
    Option<AnalyticalId>,
    Option<AnalyticalId>,
    Option<Name>,
    Option<Name>,
    Basis<Timestamp>,
    Name,
);

fn limit_stream_key(observation: &ProviderLimitObservation) -> LimitStreamKey {
    (
        observation.evidence.source,
        observation.owner_thread.clone(),
        observation.owner_request.clone(),
        observation.limit_name,
        observation.window,
    )
}

fn limit_sort_key(observation: &ProviderLimitObservation) -> LimitSortKey {
    (
        observation.evidence,
        observation.owner_thread.clone(),
        observation.owner_request.clone(),
        observation.limit_name,
        observation.window,
        observation.observed_at.clone(),
        observation.native,
    )
}

/// Sorts observations into canonical order and removes re-reads in place: identical
/// observations count once, and a later observation of a kept record location with
/// different content is diagnosed and dropped.
fn dedupe_rereads(
    observations: &mut Vec<RequestObservation>,
    diagnostics: &mut Vec<Diagnostic>,
    coverage: &mut ReconcileCoverage,
) {
    // Key order and repeated keys do not change the content of a record.
    for observation in observations.iter_mut() {
        observation.keys.sort_dedup();
    }
    // Observations that compare equal are identical, so an in-place unstable sort gives
    // the stable sort's result without its buffer of half the observations.
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
                [observation.evidence],
                "one record location was observed with different content",
            ));
            return false;
        }
        if observation.role == ObservationRole::Copy {
            coverage.copies = coverage.copies.saturating_add(1);
        }
        previous = Some(observation.evidence);
        true
    });
}

/// Registers an observation's canonical keys, linking them to each other, and gives an
/// observation without keys its artifact-local key. Returns the node of its first key.
fn resolve_identities(
    observation: &mut RequestObservation,
    graph: &mut KeyGraph,
    sources: &SourceTable,
) -> Result<u32, ReconcileError> {
    let mut first = None;
    for key in &observation.keys {
        if key.id.prefix() != IdPrefix::Request {
            return Err(ReconcileError::WrongPrefix {
                evidence: observation.evidence,
                prefix: key.id.prefix(),
            });
        }
        let node = graph.register(key)?;
        match first {
            None => first = Some(node),
            Some(first) => graph.link(first, node),
        }
    }
    if let Some(first) = first {
        return Ok(first);
    }
    let local = artifact_local(&observation.evidence, sources)?;
    let node = graph.register(&local)?;
    observation.keys.push(local);
    Ok(node)
}

fn artifact_local(
    evidence: &EvidenceRef,
    sources: &SourceTable,
) -> Result<DerivedKey, ReconcileError> {
    let source = sources.get(evidence.source).ok_or(ReconcileError::OffsetOutOfRange(*evidence))?;
    Ok(artifact_local_key(IdPrefix::Request, source, evidence.offset)
        .ok_or(ReconcileError::OffsetOutOfRange(*evidence))?
        .derive()?)
}

/// Frees what an observation owns once its request is built; its inline fields stay.
fn release_payload(observation: &mut RequestObservation) {
    observation.keys = InlineList::new();
    observation.model_usage = InlineList::new();
    observation.invariants = InlineList::new();
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

/// Revision-invariant fields on which the group's observations disagree.
fn conflicting_fields(members: &[&RequestObservation]) -> BTreeSet<String> {
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

fn build_request(
    observations: &[&RequestObservation],
    split: bool,
    selector: &dyn RevisionSelector,
    graph: &mut KeyGraph,
    diagnostics: &mut Vec<Diagnostic>,
    sources: &SourceTable,
) -> Result<Option<Request>, ReconcileError> {
    // A split part has only its artifact-local ID; otherwise every key of every member
    // counts, and a member without keys already carries its artifact-local key.
    let linked = if split {
        let mut local_keys = Vec::with_capacity(observations.len());
        for observation in observations {
            let key = artifact_local(&observation.evidence, sources)?;
            graph.register(&key)?;
            local_keys.push(key);
        }
        resolve_compact_set(&local_keys)
    } else {
        resolve_compact_set(observations.iter().flat_map(|observation| observation.keys.iter()))
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
        let choice = selector.select(&revisions);
        let Some(chosen) = revisions.get(choice.selected) else {
            return Err(ReconcileError::InvalidRevisionChoice {
                rule: selector.rule(),
                selected: choice.selected,
                count: revisions.len(),
            });
        };
        if !choice.disagreements.is_empty() {
            let mut disagreements = choice.disagreements.clone();
            disagreements.sort();
            disagreements.dedup();
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::RevisionDisagreement,
                Some(id.clone()),
                revisions.iter().map(|r| r.evidence),
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
        let selected = usage.as_ref().map(|_| *chosen);
        (usage, selected)
    };

    let counting = if originals.is_empty() {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::CopyWithoutOriginal,
            Some(id.clone()),
            observations.iter().map(|o| o.evidence),
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
            .map(|o| o.evidence)
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
                .map(|o| o.evidence),
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
            originals.iter().filter(|o| o.model.is_some()).map(|o| o.evidence),
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
                        evidence.extend(request.evidence().iter().copied());
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

#[cfg(test)]
mod tests;
