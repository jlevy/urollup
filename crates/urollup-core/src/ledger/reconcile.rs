//! The generic reconciliation engine: observations to logical requests (design §3.3).
//!
//! Adapters emit [`RequestObservation`]s; [`reconcile`] merges them into one ledger before
//! any aggregation. The steps, each computed from sorted data so the ledger never depends
//! on the order files were read or observations arrived:
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
    AccountAttribution, Counting, ModelBasis, ModelName, Ownership, Request, RevisionStatus,
    SelectedUsage, UsageRevision,
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
    /// Request observations, in any order.
    pub requests: Vec<RequestObservation>,
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
    /// Logical requests by canonical ID.
    pub requests: BTreeMap<AnalyticalId, Request>,
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

/// Reconciles request observations into a ledger; see the module documentation.
pub fn reconcile(
    input: ReconcileInput,
    selector: &dyn RevisionSelector,
) -> Result<Ledger, ReconcileError> {
    let ReconcileInput { requests, mut links, mut gaps, mut diagnostics } = input;
    let mut coverage =
        ReconcileCoverage { observations: count(requests.len()), ..ReconcileCoverage::default() };

    let observations = dedupe_rereads(requests, &mut diagnostics, &mut coverage);
    let mut registry = IdentityRegistry::new();
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

    diagnostics.sort();
    diagnostics.dedup();
    gaps.sort();
    gaps.dedup();
    Ok(Ledger { requests, candidate_sets, diagnostics, gaps, coverage })
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
            revision: UsageRevision { evidence: chosen.evidence.clone(), usage },
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
                r.usage.clone().map(|usage| UsageRevision { evidence: r.evidence.clone(), usage })
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
