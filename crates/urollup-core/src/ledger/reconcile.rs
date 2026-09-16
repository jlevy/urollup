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
//!    Ownership comes from proven owners, then candidates; conflicting accounts and
//!    served models are diagnosed, not split.
//! 5. **Candidate sets:** requests that share an adapter-declared candidate token, or were
//!    split from one conflicting key, form a candidate set. It counts the member with the
//!    strongest identity basis, then the lowest ID, and marks the others
//!    [`Counting::Unresolved`], which totals never add.

use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;

use super::coverage::{CoverageGap, ReconcileCoverage};
use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::entities::{
    AccountAttribution, Basis, Confidence, Counting, ModelBasis, ModelName, ModelUsage, Ownership,
    ProviderLimitObservation, Relationship, RelationshipKind, Request, RevisionStatus,
    SelectedUsage, Thread, ToolAction, UsageRevision,
};
use super::identity::{AnalyticalId, IdPrefix, IdentityError, IdentityRegistry};
use super::linking::{LinkGraph, ResolvedKey, resolve_linked_set};
use super::scope::{ScopedKey, artifact_local_key};
use super::tokens::TokenUsage;
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
    /// These threads may own the request, without proof.
    Candidates(BTreeSet<AnalyticalId>),
    /// The record says nothing about ownership.
    None,
}

/// One request record as an adapter decoded it.
///
/// The derived order starts with the evidence reference, which makes it the canonical
/// observation order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestObservation {
    /// Where the record is.
    pub evidence: EvidenceRef,
    /// The dialect registry token.
    pub dialect: String,
    /// Every `req-` key the adapter could build for the record, in any order.
    pub keys: Vec<ScopedKey>,
    /// Original or copy.
    pub role: ObservationRole,
    /// Owner evidence.
    pub owner: OwnerEvidence,
    /// The record's usage, when it carries any.
    pub usage: Option<TokenUsage>,
    /// This record's usage split by model.
    pub model_usage: Vec<ModelUsage>,
    /// A native revision sequence, when the dialect orders revisions.
    pub sequence: Option<u64>,
    /// Revision-invariant fields; observations sharing a key must agree on every field
    /// both carry.
    pub invariants: BTreeMap<String, String>,
    /// Tokens naming candidate sets: observations that may be one request but share no
    /// key, such as a digest of copy-invariant content.
    pub candidate_tokens: BTreeSet<String>,
    /// The native request ID.
    pub native_request_id: Option<String>,
    /// The native response ID.
    pub native_response_id: Option<String>,
    /// The model, when recorded.
    pub model: Option<ModelName>,
    /// The reasoning effort, when recorded.
    pub effort: Option<String>,
    /// The stable account identifier, when recorded.
    pub account: Option<String>,
    /// The record's timestamp.
    pub timestamp: Option<Timestamp>,
}

impl RequestObservation {
    /// An original observation with no keys, owner, usage or properties yet.
    pub fn new(evidence: EvidenceRef, dialect: impl Into<String>) -> Self {
        Self {
            evidence,
            dialect: dialect.into(),
            keys: Vec::new(),
            role: ObservationRole::Original,
            owner: OwnerEvidence::None,
            usage: None,
            model_usage: Vec::new(),
            sequence: None,
            invariants: BTreeMap::new(),
            candidate_tokens: BTreeSet::new(),
            native_request_id: None,
            native_response_id: None,
            model: None,
            effort: None,
            account: None,
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
    /// A stable rule name recorded with the selected usage.
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
}

/// The reconciled request ledger.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Ledger {
    /// Logical threads by canonical ID.
    pub threads: BTreeMap<AnalyticalId, Thread>,
    /// Native relationships in canonical order.
    pub relationships: Vec<Relationship>,
    /// Logical requests by canonical ID.
    pub requests: BTreeMap<AnalyticalId, Request>,
    /// Logical tool actions by canonical ID.
    pub tool_actions: BTreeMap<AnalyticalId, ToolAction>,
    /// Provider limit observations in canonical order.
    pub limit_observations: Vec<ProviderLimitObservation>,
    /// Candidate sets with more than one member.
    pub candidate_sets: Vec<BTreeSet<AnalyticalId>>,
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
}

/// One observation with its derived identities.
struct Resolved {
    observation: RequestObservation,
    keys: Vec<ResolvedKey>,
    local: ResolvedKey,
}

/// Reconciles normalized observations into a ledger; see the module documentation.
pub fn reconcile(
    input: ReconcileInput,
    selector: &dyn RevisionSelector,
) -> Result<Ledger, ReconcileError> {
    let ReconcileInput {
        threads,
        relationships,
        requests,
        tool_actions,
        limit_observations,
        mut links,
        mut gaps,
        mut diagnostics,
    } = input;
    let mut coverage =
        ReconcileCoverage { observations: count(requests.len()), ..ReconcileCoverage::default() };

    let mut registry = IdentityRegistry::new();
    let threads = reconcile_threads(threads, &mut registry, &mut diagnostics)?;
    let thread_ids = canonical_thread_ids(&threads);
    let relationships = reconcile_relationships(relationships, &thread_ids)?;
    let requests = canonicalize_request_owners(requests, &thread_ids);

    let observations = dedupe_rereads(requests, &mut diagnostics, &mut coverage);
    let resolved = resolve_identities(observations, &mut registry)?;

    // Link observations sharing a key ID, and IDs joined by lineage evidence.
    let mut graph = LinkGraph::new();
    for item in &resolved {
        let first = item.keys.first().unwrap_or(&item.local).id().clone();
        graph.insert(&first);
        for key in &item.keys {
            graph.link(&first, key.id());
        }
    }
    links.sort();
    for link in &links {
        graph.link(&link.a, &link.b);
    }
    let mut groups: BTreeMap<AnalyticalId, Vec<&Resolved>> = BTreeMap::new();
    for item in &resolved {
        let first = item.keys.first().unwrap_or(&item.local).id();
        groups.entry(graph.find(first)).or_default().push(item);
    }

    let mut requests = BTreeMap::new();
    let mut candidates = LinkGraph::new();
    let mut tokens: BTreeMap<&str, BTreeSet<AnalyticalId>> = BTreeMap::new();
    for members in groups.values() {
        let split = conflicting_fields(members);
        let parts: Vec<Vec<&Resolved>> = if split.is_empty() {
            vec![members.clone()]
        } else {
            coverage.conflicting_keys = coverage.conflicting_keys.saturating_add(1);
            members.iter().map(|member| vec![*member]).collect()
        };
        let mut split_ids = Vec::new();
        for part in parts {
            let Some(request) =
                build_request(&part, !split.is_empty(), selector, &mut diagnostics)?
            else {
                continue;
            };
            let id = request.id().clone();
            candidates.insert(&id);
            for member in &part {
                for token in &member.observation.candidate_tokens {
                    tokens.entry(token).or_default().insert(id.clone());
                }
            }
            split_ids.push(id.clone());
            requests.insert(id, request);
        }
        if !split.is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ConflictingSharedKey,
                None,
                members.iter().map(|member| member.observation.evidence.clone()),
                format!("observations sharing a key disagree on {}", join(split.iter())),
            ));
            for pair in split_ids.windows(2) {
                candidates.link(&pair[0], &pair[1]);
            }
        }
    }
    for ids in tokens.values() {
        let mut ids = ids.iter();
        if let Some(first) = ids.next() {
            for other in ids {
                candidates.link(first, other);
            }
        }
    }

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

    diagnostics.sort();
    diagnostics.dedup();
    gaps.sort();
    gaps.dedup();
    Ok(Ledger {
        threads,
        relationships,
        requests,
        tool_actions,
        limit_observations,
        candidate_sets,
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
    let mut previous: BTreeMap<LimitStreamKey, String> = BTreeMap::new();
    for observation in observations {
        let stream = limit_stream_key(&observation);
        let signature = serde_json::to_string(&observation.native).unwrap_or_default();
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

fn canonical_request_ids(
    requests: &BTreeMap<AnalyticalId, Request>,
) -> BTreeMap<AnalyticalId, AnalyticalId> {
    requests
        .iter()
        .flat_map(|(id, request)| {
            std::iter::once((id.clone(), id.clone()))
                .chain(request.aliases.iter().map(|alias| (alias.id.clone(), id.clone())))
        })
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
            OwnerEvidence::Candidates(candidates) => OwnerEvidence::Candidates(
                candidates.iter().map(|thread| canonical_id(thread, thread_ids)).collect(),
            ),
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
    String,
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
        serde_json::to_string(&observation.native).unwrap_or_default(),
    )
}

fn dedupe_rereads(
    mut observations: Vec<RequestObservation>,
    diagnostics: &mut Vec<Diagnostic>,
    coverage: &mut ReconcileCoverage,
) -> Vec<RequestObservation> {
    observations.sort();
    let before = observations.len();
    observations.dedup();
    coverage.rereads = count(before.saturating_sub(observations.len()));
    let mut kept: Vec<RequestObservation> = Vec::with_capacity(observations.len());
    for observation in observations {
        match kept.last() {
            Some(previous) if previous.evidence == observation.evidence => {
                coverage.conflicting_rereads = coverage.conflicting_rereads.saturating_add(1);
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::ConflictingReread,
                    None,
                    [observation.evidence.clone()],
                    "one record location was observed with different content",
                ));
            }
            Some(_) | None => {
                if observation.role == ObservationRole::Copy {
                    coverage.copies = coverage.copies.saturating_add(1);
                }
                kept.push(observation);
            }
        }
    }
    kept
}

fn resolve_identities(
    observations: Vec<RequestObservation>,
    registry: &mut IdentityRegistry,
) -> Result<Vec<Resolved>, ReconcileError> {
    let mut resolved = Vec::with_capacity(observations.len());
    for mut observation in observations {
        observation.keys.sort();
        observation.keys.dedup();
        let mut keys = Vec::with_capacity(observation.keys.len());
        for key in &observation.keys {
            if key.key.prefix != IdPrefix::Request {
                return Err(ReconcileError::WrongPrefix {
                    evidence: observation.evidence.clone(),
                    prefix: key.key.prefix,
                });
            }
            registry.derive(&key.key)?;
            keys.push(ResolvedKey::derive(key.clone())?);
        }
        let local_key = artifact_local_key(
            IdPrefix::Request,
            &observation.evidence.source,
            observation.evidence.offset,
        )
        .ok_or_else(|| ReconcileError::OffsetOutOfRange(observation.evidence.clone()))?;
        registry.derive(&local_key.key)?;
        let local = ResolvedKey::derive(local_key)?;
        resolved.push(Resolved { observation, keys, local });
    }
    Ok(resolved)
}

/// Revision-invariant fields on which the group's observations disagree.
fn conflicting_fields(members: &[&Resolved]) -> BTreeSet<String> {
    let mut values: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for member in members {
        for (field, value) in &member.observation.invariants {
            values.entry(field).or_default().insert(value);
        }
    }
    values
        .into_iter()
        .filter(|(_, distinct)| distinct.len() > 1)
        .map(|(field, _)| field.to_owned())
        .collect()
}

fn build_request(
    members: &[&Resolved],
    split: bool,
    selector: &dyn RevisionSelector,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Option<Request>, ReconcileError> {
    // A split part has only its artifact-local ID; otherwise every key of every member
    // counts, with the artifact-local ID standing in for a member that has no key.
    let identities = members.iter().flat_map(|member| {
        if split || member.keys.is_empty() {
            std::slice::from_ref(&member.local)
        } else {
            member.keys.as_slice()
        }
    });
    let Some(linked) = resolve_linked_set(identities) else {
        return Ok(None);
    };
    let id = linked.canonical.id.clone();
    let observations: Vec<&RequestObservation> =
        members.iter().map(|member| &member.observation).collect();
    let originals: Vec<&RequestObservation> =
        observations.iter().copied().filter(|o| o.role == ObservationRole::Original).collect();
    let revisions: Vec<&RequestObservation> =
        originals.iter().copied().filter(|o| o.usage.is_some()).collect();

    let usage = if revisions.is_empty() {
        None
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
                revisions.iter().map(|r| r.evidence.clone()),
                format!("{}: {}", selector.rule(), join(disagreements.iter())),
            ));
        }
        chosen.usage.clone().map(|usage| SelectedUsage {
            revision: UsageRevision {
                evidence: chosen.evidence.clone(),
                usage,
                model_usage: chosen.model_usage.clone(),
            },
            status: choice.status,
            rule: selector.rule(),
        })
    };
    let selected_evidence = usage.as_ref().map(|u| &u.revision.evidence);
    let selected = revisions.iter().find(|r| Some(&r.evidence) == selected_evidence);

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
        native_request_id: observations.iter().filter_map(|o| o.native_request_id.clone()).min(),
        native_response_id: observations.iter().filter_map(|o| o.native_response_id.clone()).min(),
        ownership: ownership(&observations, &id, diagnostics),
        first_seen: originals.iter().filter_map(|o| o.timestamp).min(),
        last_seen: originals.iter().filter_map(|o| o.timestamp).max(),
        model: model(&originals, selected.copied(), &id, diagnostics),
        effort: selected
            .and_then(|s| s.effort.clone())
            .or_else(|| originals.iter().filter_map(|o| o.effort.clone()).min()),
        account: account(&observations, &id, diagnostics),
        revisions: revisions
            .iter()
            .filter_map(|r| {
                r.usage.clone().map(|usage| UsageRevision {
                    evidence: r.evidence.clone(),
                    usage,
                    model_usage: r.model_usage.clone(),
                })
            })
            .collect(),
        evidence: originals.iter().map(|o| o.evidence.clone()).collect(),
        copies: observations
            .iter()
            .filter(|o| o.role == ObservationRole::Copy)
            .map(|o| o.evidence.clone())
            .collect(),
        usage,
        counting,
        identity: linked.canonical,
    }))
}

fn ownership(
    observations: &[&RequestObservation],
    id: &AnalyticalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Ownership {
    let mut proven = BTreeSet::new();
    let mut candidates = BTreeSet::new();
    for observation in observations {
        match &observation.owner {
            OwnerEvidence::Proven(thread) => {
                proven.insert(thread.clone());
            }
            OwnerEvidence::Candidates(threads) => candidates.extend(threads.iter().cloned()),
            OwnerEvidence::None => {}
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
    if let Some(thread) = proven.into_iter().next() {
        return Ownership::Owned { thread };
    }
    if candidates.is_empty() { Ownership::Unknown } else { Ownership::Ambiguous { candidates } }
}

fn account(
    observations: &[&RequestObservation],
    id: &AnalyticalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> AccountAttribution {
    let accounts: BTreeSet<String> =
        observations.iter().filter_map(|o| o.account.clone()).collect();
    if accounts.len() > 1 {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::ConflictingAccounts,
            Some(id.clone()),
            observations.iter().filter(|o| o.account.is_some()).map(|o| o.evidence.clone()),
            format!("accounts {}", join(accounts.iter())),
        ));
        return AccountAttribution::Conflicting(accounts);
    }
    accounts.into_iter().next().map_or(AccountAttribution::Unknown, AccountAttribution::Attributed)
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
    selected.and_then(|s| s.model.clone()).or_else(|| {
        originals
            .iter()
            .filter_map(|o| o.model.clone())
            .min_by(|a, b| (a.basis, &a.name).cmp(&(b.basis, &b.name)))
    })
}

fn resolve_candidate_sets(
    candidates: &LinkGraph,
    requests: &mut BTreeMap<AnalyticalId, Request>,
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
                        evidence.extend(request.evidence.iter().cloned());
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
