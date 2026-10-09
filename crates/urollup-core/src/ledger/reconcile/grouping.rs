//! Linked sets of request observations, found by sorting their keys.
//!
//! Every key of every observation becomes a 29-byte [`KeyEntry`], and the entries are
//! sorted by ID. Entries that share an ID join their observations in a union-find over
//! observation indices, and an ID whose entries disagree on their further digest bits is a
//! collision. An observation's own keys need no links, because they share its node.
//!
//! This gives the sets, set order and errors of registering each key in a map from IDs to
//! union-find nodes, one key at a time in canonical order, in about half the memory.

use super::{LineageLink, ReconcileError, RequestObservation, artifact_local, index_u32};
use crate::ledger::chunked::ChunkedVec;
use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityError};

/// One registered key: its ID, the observation that carries it, and its check bits.
///
/// Every field is byte-aligned, so an entry takes 29 bytes. The derived order sorts by ID
/// and then by observation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct KeyEntry {
    id: AnalyticalId,
    /// The observation's index as big-endian bytes, which order as the number they spell.
    observation: [u8; 4],
    check: [u8; 8],
}

const _: () = assert!(size_of::<KeyEntry>() == 29);

impl KeyEntry {
    fn observation(&self) -> usize {
        u32::from_be_bytes(self.observation) as usize
    }
}

/// Observations arranged into linked sets.
pub(super) struct Grouping {
    /// Each position's set rank. Observations are permuted so that every set is one run of
    /// equal ranks, runs in order of their set's lowest ID and members in canonical order.
    pub(super) ranks: Vec<u32>,
    /// Every registered key, sorted by ID.
    keys: Vec<KeyEntry>,
}

impl Grouping {
    /// The check bits registered for `id`, when an observation carries a key with that ID.
    pub(super) fn registered_check(&self, id: &AnalyticalId) -> Option<[u8; 8]> {
        let index = self.keys.binary_search_by(|entry| entry.id.cmp(id)).ok()?;
        Some(self.keys[index].check)
    }

    /// Frees the registered keys, keeping the ranks.
    pub(super) fn into_ranks(self) -> Vec<u32> {
        self.ranks
    }
}

/// Registers every observation key, joins observations that share a key ID or a lineage
/// link into sets, and permutes `observations` into group order.
///
/// Each observation's keys are sorted, and an observation without keys gets its
/// artifact-local key. Errors are those that registering keys one at a time in canonical
/// order meets first: a key with a wrong prefix, a keyless observation beyond an
/// artifact-local key's offsets, or a key whose ID an earlier key derived with different
/// check bits.
pub(super) fn group(
    observations: &mut ChunkedVec<RequestObservation>,
    links: &[LineageLink],
) -> Result<Grouping, ReconcileError> {
    let keys = register_keys(observations)?;
    let order = set_order(&keys, links, observations.len());
    let mut permutation: Vec<u32> = order.iter().map(|&(_, index)| index).collect();
    observations.permute(&mut permutation);
    drop(permutation);
    let ranks = order.into_iter().map(|(rank, _)| rank).collect();
    Ok(Grouping { ranks, keys })
}

/// Sorts each observation's keys, gives a keyless observation its artifact-local key, and
/// returns every key sorted by ID.
fn register_keys(
    observations: &mut ChunkedVec<RequestObservation>,
) -> Result<Vec<KeyEntry>, ReconcileError> {
    // Sorting removes only repeated keys, so this bounds the entries without regrowth.
    let capacity = observations.iter().map(|observation| observation.keys.len().max(1)).sum();
    let mut keys = Vec::with_capacity(capacity);
    // The first failure in registration order, with the observation and key position it
    // stopped at; no key from that position on was registered.
    let mut failure = None;
    for (index, observation) in observations.iter_mut().enumerate() {
        observation.keys.sort_dedup();
        let tag = index_u32(index).to_be_bytes();
        for (position, key) in observation.keys.iter().enumerate() {
            if key.id.prefix() != IdPrefix::Request {
                let error = ReconcileError::WrongPrefix {
                    evidence: observation.evidence.clone(),
                    prefix: key.id.prefix(),
                };
                failure = Some(((index, position), error));
                break;
            }
            keys.push(KeyEntry { id: key.id.clone(), observation: tag, check: key.check });
        }
        if failure.is_some() {
            break;
        }
        if observation.keys.is_empty() {
            match artifact_local(&observation.evidence) {
                Ok(local) => {
                    keys.push(KeyEntry {
                        id: local.id.clone(),
                        observation: tag,
                        check: local.check,
                    });
                    observation.keys.push(local);
                }
                Err(error) => {
                    failure = Some(((index, 0), error));
                    break;
                }
            }
        }
    }
    keys.sort_unstable();
    let stop = failure.as_ref().map(|(position, _)| *position);
    if let Some(collision) = first_collision(&keys, observations, stop) {
        return Err(collision);
    }
    match failure {
        Some((_, error)) => Err(error),
        None => Ok(keys),
    }
}

/// The digest collision that registering keys one at a time in canonical order meets
/// first, among the keys registered before `stop`.
///
/// Registration compares each key with the first key registered under its ID. Sorted
/// entries put an ID's keys together, so an ID whose entries all agree needs nothing more.
/// Only an ID whose entries disagree, which takes a digest collision, looks up key
/// positions within its observations to find the key registration would have failed on.
fn first_collision(
    keys: &[KeyEntry],
    observations: &ChunkedVec<RequestObservation>,
    stop: Option<(usize, usize)>,
) -> Option<ReconcileError> {
    let mut first: Option<((usize, usize), &AnalyticalId)> = None;
    for run in keys.chunk_by(|left, right| left.id == right.id) {
        let id = &run[0].id;
        if run.iter().all(|entry| entry.check == run[0].check) {
            continue;
        }
        let mut registered = None;
        let mut collision = None;
        let mut previous = None;
        'registration: for entry in run {
            let index = entry.observation();
            if previous == Some(index) {
                continue;
            }
            previous = Some(index);
            for (position, key) in observations[index].keys.iter().enumerate() {
                if stop.is_some_and(|stop| (index, position) >= stop) {
                    break 'registration;
                }
                if key.id != *id {
                    continue;
                }
                match registered {
                    None => registered = Some(key.check),
                    Some(check) if check != key.check => {
                        collision = Some((index, position));
                        break 'registration;
                    }
                    Some(_) => {}
                }
            }
        }
        if let Some(position) = collision {
            if first.is_none_or(|(earliest, _)| position < earliest) {
                first = Some((position, id));
            }
        }
    }
    first.map(|(_, id)| IdentityError::DigestCollision { id: id.clone() }.into())
}

/// Every observation's set rank with its index, sorted into group order.
///
/// Ranks follow ID order, so sorting by rank orders sets by their lowest ID. A lineage link
/// endpoint that no observation carries still joins sets and can be a set's lowest ID, as
/// it could as a node of a map from IDs; links have no producer today, so this costs
/// nothing in practice.
fn set_order(keys: &[KeyEntry], links: &[LineageLink], count: usize) -> Vec<(u32, u32)> {
    let find_key = |id: &AnalyticalId| keys.binary_search_by(|entry| entry.id.cmp(id)).ok();
    let mut link_only: Vec<&AnalyticalId> = links
        .iter()
        .flat_map(|link| [&link.a, &link.b])
        .filter(|id| find_key(id).is_none())
        .collect();
    link_only.sort_unstable();
    link_only.dedup();

    // A key's rank is the position of its ID's first entry plus the link-only IDs below it;
    // a link-only ID's rank is its position plus the entries below it. Ranks are then
    // distinct per ID and ordered as the IDs are.
    let mut forest = Forest::new(count + link_only.len());
    let mut first = 0;
    for (position, entry) in keys.iter().enumerate() {
        if keys[first].id != entry.id {
            first = position;
        }
        let node = entry.observation();
        if forest.least[node] == u32::MAX {
            // Entries arrive in ID order, so an observation's first entry has its lowest ID.
            forest.least[node] =
                index_u32(first + link_only.partition_point(|other| **other < entry.id));
        }
        if first != position {
            forest.union(node, keys[first].observation());
        }
    }
    for (offset, id) in link_only.iter().enumerate() {
        forest.least[count + offset] =
            index_u32(offset + keys.partition_point(|entry| entry.id < **id));
    }
    let node = |id: &AnalyticalId| {
        find_key(id).map_or_else(
            || count + link_only.binary_search(&id).unwrap_or_else(|position| position),
            |index| keys[index].observation(),
        )
    };
    for link in links {
        let (a, b) = (node(&link.a), node(&link.b));
        forest.union(a, b);
    }

    let mut order: Vec<(u32, u32)> =
        (0..count).map(|index| (forest.set_rank(index), index_u32(index))).collect();
    drop(forest);
    order.sort_unstable();
    order
}

/// A union-find over observations and link-only IDs whose roots carry their set's lowest
/// rank.
struct Forest {
    parent: Vec<u32>,
    least: Vec<u32>,
}

impl Forest {
    fn new(nodes: usize) -> Self {
        Self { parent: (0..index_u32(nodes)).collect(), least: vec![u32::MAX; nodes] }
    }

    fn find(&mut self, node: usize) -> usize {
        let mut root = node;
        while self.parent[root] as usize != root {
            root = self.parent[root] as usize;
        }
        let mut current = node;
        while current != root {
            let next = self.parent[current] as usize;
            self.parent[current] = index_u32(root);
            current = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a == b {
            return;
        }
        let (root, child) = if self.least[a] <= self.least[b] { (a, b) } else { (b, a) };
        self.parent[child] = index_u32(root);
    }

    fn set_rank(&mut self, node: usize) -> u32 {
        let root = self.find(node);
        self.least[root]
    }
}
