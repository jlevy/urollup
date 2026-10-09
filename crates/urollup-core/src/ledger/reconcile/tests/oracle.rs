//! The request reconciliation that sorted key grouping and backward building replaced,
//! kept as a test oracle.
//!
//! It registers each key in a map from IDs to union-find nodes, one key at a time in
//! canonical order, and builds requests forwards over one contiguous vector. It shares
//! only the per-request helpers with the engine, so comparing the two checks grouping, set
//! order, the build order and error precedence.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::ledger::coverage::ReconcileCoverage;
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode, compact};
use crate::ledger::entities::{Counting, Requests};
use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityError};
use crate::ledger::linking::LinkGraph;
use crate::ledger::reconcile::{
    Ledger, LineageLink, ObservationRole, ReconcileError, ReconcileInput, RequestObservation,
    RevisionSelector, artifact_local, build_request, resolve_candidate_sets,
};
use crate::ledger::scope::DerivedKey;
use crate::sources::evidence::EvidenceRef;

/// Request key IDs as union-find nodes, with the further digest bits that catch two
/// different keys deriving one ID. A set's root is always its lowest ID.
#[derive(Default)]
pub(super) struct KeyGraph {
    nodes: HashMap<AnalyticalId, u32>,
    ids: Vec<AnalyticalId>,
    checks: Vec<Option<[u8; 8]>>,
    parent: Vec<u32>,
}

impl KeyGraph {
    fn node(&mut self, id: &AnalyticalId) -> u32 {
        if let Some(node) = self.nodes.get(id) {
            return *node;
        }
        let node = u32::try_from(self.ids.len()).unwrap();
        self.nodes.insert(id.clone(), node);
        self.ids.push(id.clone());
        self.checks.push(None);
        self.parent.push(node);
        node
    }

    pub(super) fn register(&mut self, key: &DerivedKey) -> Result<u32, IdentityError> {
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

    pub(super) fn id(&self, node: u32) -> &AnalyticalId {
        &self.ids[node as usize]
    }

    pub(super) fn find(&mut self, node: u32) -> u32 {
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

    pub(super) fn link(&mut self, a: u32, b: u32) {
        let (root_a, root_b) = (self.find(a), self.find(b));
        match self.id(root_a).cmp(self.id(root_b)) {
            std::cmp::Ordering::Less => self.parent[root_b as usize] = root_a,
            std::cmp::Ordering::Greater => self.parent[root_a as usize] = root_b,
            std::cmp::Ordering::Equal => {}
        }
    }
}

/// Sorts observations and removes re-reads with vector operations.
pub(super) fn dedupe_rereads(
    observations: &mut Vec<RequestObservation>,
    diagnostics: &mut Vec<Diagnostic>,
    coverage: &mut ReconcileCoverage,
) {
    observations.sort();
    let before = observations.len();
    observations.dedup();
    coverage.rereads = u64::try_from(before - observations.len()).unwrap();
    let mut previous: Option<EvidenceRef> = None;
    observations.retain(|observation| {
        if previous.as_ref() == Some(&observation.evidence) {
            coverage.conflicting_rereads += 1;
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ConflictingReread,
                None,
                [observation.evidence.clone()],
                "one record location was observed with different content",
            ));
            return false;
        }
        if observation.role == ObservationRole::Copy {
            coverage.copies += 1;
        }
        previous = Some(observation.evidence.clone());
        true
    });
}

fn resolve_identities(
    observation: &mut RequestObservation,
    graph: &mut KeyGraph,
) -> Result<u32, ReconcileError> {
    observation.keys.sort_dedup();
    let mut first = None;
    for key in &observation.keys {
        if key.id.prefix() != IdPrefix::Request {
            return Err(ReconcileError::WrongPrefix {
                evidence: observation.evidence.clone(),
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
    let local = artifact_local(&observation.evidence)?;
    let node = graph.register(&local)?;
    observation.keys.push(local);
    Ok(node)
}

/// Linked sets of deduplicated observations as the key graph orders them: sets by lowest
/// ID, members in canonical order.
pub(super) fn linked_sets(
    mut observations: Vec<RequestObservation>,
    links: &[LineageLink],
) -> Result<Vec<Vec<RequestObservation>>, ReconcileError> {
    let mut graph = KeyGraph::default();
    let mut order = Vec::with_capacity(observations.len());
    for (index, observation) in observations.iter_mut().enumerate() {
        order.push((resolve_identities(observation, &mut graph)?, index));
    }
    let mut links = links.to_vec();
    links.sort();
    for link in &links {
        let (a, b) = (graph.node(&link.a), graph.node(&link.b));
        graph.link(a, b);
    }
    for entry in &mut order {
        entry.0 = graph.find(entry.0);
    }
    order.sort_unstable_by(|(left_root, left), (right_root, right)| {
        graph.id(*left_root).cmp(graph.id(*right_root)).then(left.cmp(right))
    });
    let mut slots: Vec<Option<RequestObservation>> = observations.into_iter().map(Some).collect();
    Ok(order
        .chunk_by(|left, right| left.0 == right.0)
        .map(|set| set.iter().map(|(_, index)| slots[*index].take().unwrap()).collect())
        .collect())
}

/// The fields a set's observations disagree on, without the engine's shortcut.
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

/// Reconciles an input of request observations and links only.
pub(super) fn reconcile(
    input: ReconcileInput,
    selector: &dyn RevisionSelector,
) -> Result<Ledger, ReconcileError> {
    let ReconcileInput { requests, links, mut diagnostics, .. } = input;
    let mut observations: Vec<RequestObservation> = requests.iter().cloned().collect();
    let mut coverage = ReconcileCoverage {
        observations: u64::try_from(observations.len()).unwrap(),
        ..ReconcileCoverage::default()
    };
    dedupe_rereads(&mut observations, &mut diagnostics, &mut coverage);

    let mut graph = KeyGraph::default();
    let mut first_keys = Vec::with_capacity(observations.len());
    for observation in &mut observations {
        first_keys.push(resolve_identities(observation, &mut graph)?);
    }
    let mut links = links;
    links.sort();
    for link in &links {
        let (a, b) = (graph.node(&link.a), graph.node(&link.b));
        graph.link(a, b);
    }
    let mut order: Vec<(u32, u32)> = Vec::with_capacity(observations.len());
    for (index, first) in first_keys.into_iter().enumerate() {
        order.push((graph.find(first), u32::try_from(index).unwrap()));
    }
    order.sort_unstable_by(|(left_root, left), (right_root, right)| {
        graph.id(*left_root).cmp(graph.id(*right_root)).then(left.cmp(right))
    });

    let mut requests = Vec::new();
    let mut candidates = LinkGraph::new();
    for group in order.chunk_by(|left, right| left.0 == right.0) {
        let members: Vec<&RequestObservation> =
            group.iter().map(|(_, index)| &observations[*index as usize]).collect();
        let split = conflicting_fields(&members);
        if split.is_empty() {
            requests.extend(build_request(&members, None, selector, &mut diagnostics)?);
            continue;
        }
        coverage.conflicting_keys += 1;
        let mut split_ids = Vec::new();
        for member in &members {
            let key = artifact_local(&member.evidence)?;
            graph.register(&key)?;
            let Some(request) = build_request(&[*member], Some(&key), selector, &mut diagnostics)?
            else {
                continue;
            };
            split_ids.push(request.id().clone());
            requests.push(request);
        }
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::ConflictingSharedKey,
            None,
            members.iter().map(|member| member.evidence.clone()),
            format!("observations sharing a key disagree on {}", super::super::join(split.iter())),
        ));
        for pair in split_ids.windows(2) {
            candidates.link(&pair[0], &pair[1]);
        }
    }

    let mut requests = Requests::from_unsorted(requests);
    let candidate_sets = resolve_candidate_sets(&candidates, &mut requests, &mut diagnostics);
    let count = |n: usize| u64::try_from(n).unwrap();
    coverage.candidate_sets = count(candidate_sets.len());
    coverage.requests = count(requests.len());
    coverage.copy_only_requests =
        count(requests.values().filter(|r| r.counting == Counting::CopyOnly).count());
    coverage.unresolved_requests = count(
        requests.values().filter(|r| matches!(r.counting, Counting::Unresolved { .. })).count(),
    );
    coverage.requests_without_usage =
        count(requests.values().filter(|r| r.usage.is_none()).count());
    Ok(Ledger {
        requests,
        candidate_sets,
        revision_rule: selector.rule(),
        diagnostics: compact(diagnostics),
        coverage,
        ..Ledger::default()
    })
}
