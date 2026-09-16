//! Linked identity sets (design §3.6, Identity Basis and Linking).
//!
//! Observations are linked by a shared key or by lineage evidence. A linked set takes the
//! ID of its highest-precedence key, then the lowest ID, and keeps every other ID as an
//! alias. Both [`LinkGraph`] and [`resolve_linked_set`] compute from the set alone, so the
//! result never depends on the order links or members arrive in.

use std::collections::{BTreeMap, BTreeSet};

use super::identity::{AnalyticalId, IdentityError, StoredIdentity};
use super::scope::{IdentityBasis, ScopedKey};

/// A key with its derived ID.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResolvedKey {
    /// Precedence of the key's kind; 0 is the highest.
    pub precedence: u8,
    /// The basis the key establishes.
    pub basis: IdentityBasis,
    /// The derived ID with its key.
    pub identity: StoredIdentity,
}

impl ResolvedKey {
    /// Derives the ID of a scoped key.
    pub fn derive(key: ScopedKey) -> Result<Self, IdentityError> {
        Ok(Self {
            precedence: key.precedence,
            basis: key.basis,
            identity: StoredIdentity::derive(key.key)?,
        })
    }

    /// The derived ID.
    pub fn id(&self) -> &AnalyticalId {
        &self.identity.id
    }
}

/// The identity a linked set resolves to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LinkedIdentity {
    /// The chosen ID and its key.
    pub canonical: StoredIdentity,
    /// The basis of the chosen key.
    pub basis: IdentityBasis,
    /// Every other distinct ID in the set with its key, in ID order.
    pub aliases: Vec<StoredIdentity>,
}

/// Resolves a linked set: the highest-precedence key's ID, then the lowest ID; the rest
/// become aliases. Returns `None` for an empty set.
pub fn resolve_linked_set<'a>(
    members: impl IntoIterator<Item = &'a ResolvedKey>,
) -> Option<LinkedIdentity> {
    let unique: BTreeSet<(u8, IdentityBasis, &StoredIdentity)> =
        members.into_iter().map(|key| (key.precedence, key.basis, &key.identity)).collect();
    let mut ranked = unique.into_iter();
    let (_, basis, canonical) = ranked.next()?;
    let mut aliases: Vec<StoredIdentity> = ranked
        .filter(|(_, _, other)| other.id != canonical.id)
        .map(|(_, _, other)| other.clone())
        .collect();
    aliases.sort();
    aliases.dedup_by(|a, b| a.id == b.id);
    Some(LinkedIdentity { canonical: canonical.clone(), basis, aliases })
}

/// A union-find over IDs whose components are independent of link order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LinkGraph {
    parent: BTreeMap<AnalyticalId, AnalyticalId>,
}

impl LinkGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `id` as a member, alone if it has no links.
    pub fn insert(&mut self, id: &AnalyticalId) {
        if !self.parent.contains_key(id) {
            self.parent.insert(id.clone(), id.clone());
        }
    }

    /// Links two IDs into one set.
    pub fn link(&mut self, a: &AnalyticalId, b: &AnalyticalId) {
        self.insert(a);
        self.insert(b);
        let root_a = self.find(a);
        let root_b = self.find(b);
        // The lower ID always becomes the root, so roots do not depend on link order.
        match root_a.cmp(&root_b) {
            std::cmp::Ordering::Less => {
                self.parent.insert(root_b, root_a);
            }
            std::cmp::Ordering::Greater => {
                self.parent.insert(root_a, root_b);
            }
            std::cmp::Ordering::Equal => {}
        }
    }

    /// The root of `id`'s set, which is the lowest ID in it; an unknown ID is its own root.
    pub fn find(&self, id: &AnalyticalId) -> AnalyticalId {
        let mut current = id;
        while let Some(parent) = self.parent.get(current) {
            if parent == current {
                break;
            }
            current = parent;
        }
        current.clone()
    }

    /// Every set, each keyed by its lowest ID.
    pub fn components(&self) -> BTreeMap<AnalyticalId, BTreeSet<AnalyticalId>> {
        let mut sets: BTreeMap<AnalyticalId, BTreeSet<AnalyticalId>> = BTreeMap::new();
        for id in self.parent.keys() {
            sets.entry(self.find(id)).or_default().insert(id.clone());
        }
        sets
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{LinkGraph, ResolvedKey, resolve_linked_set};
    use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent};
    use crate::ledger::scope::{IdentityBasis, ScopedKey};
    use crate::test_support::shuffle;

    fn resolved(precedence: u8, native_id: &str) -> ResolvedKey {
        let basis = if precedence < 2 { IdentityBasis::Native } else { IdentityBasis::Fallback };
        ResolvedKey::derive(ScopedKey {
            precedence,
            basis,
            key: IdentityKey::new(
                IdPrefix::Request,
                format!("kind-{precedence}"),
                vec![KeyComponent::text(native_id)],
            ),
        })
        .unwrap()
    }

    fn id(n: u32) -> AnalyticalId {
        IdentityKey::new(IdPrefix::Request, "n", vec![KeyComponent::Integer(i64::from(n))])
            .derive_id()
            .unwrap()
    }

    #[test]
    fn the_highest_precedence_key_wins_even_with_a_higher_id() {
        let members = [resolved(3, "a"), resolved(0, "b"), resolved(3, "c")];
        let linked = resolve_linked_set(&members).unwrap();
        assert_eq!(linked.canonical, members[1].identity);
        assert_eq!(linked.basis, IdentityBasis::Native);
        assert_eq!(linked.aliases.len(), 2);
        assert!(linked.aliases.windows(2).all(|pair| pair[0].id < pair[1].id));
    }

    #[test]
    fn equal_precedence_takes_the_lowest_id() {
        let members = [resolved(1, "x"), resolved(1, "y"), resolved(1, "z")];
        let lowest = members.iter().map(ResolvedKey::id).min().unwrap().clone();
        assert_eq!(resolve_linked_set(&members).unwrap().canonical.id, lowest);
    }

    #[test]
    fn repeated_members_are_one_member() {
        let members = [resolved(0, "a"), resolved(0, "a")];
        let linked = resolve_linked_set(&members).unwrap();
        assert!(linked.aliases.is_empty());
        assert!(resolve_linked_set(&[]).is_none());
    }

    proptest! {
        #[test]
        fn a_linked_set_resolves_the_same_in_any_order(
            specs in prop::collection::vec((0u8..4, 0u8..6), 1..8),
            seed in any::<u64>(),
        ) {
            let members: Vec<ResolvedKey> =
                specs.iter().map(|(precedence, n)| resolved(*precedence, &n.to_string())).collect();
            let mut shuffled = members.clone();
            shuffle(&mut shuffled, seed);
            prop_assert_eq!(resolve_linked_set(&members), resolve_linked_set(&shuffled));
        }

        #[test]
        fn link_graph_components_do_not_depend_on_link_order(
            links in prop::collection::vec((0u32..12, 0u32..12), 0..20),
            seed in any::<u64>(),
        ) {
            let mut forward = LinkGraph::new();
            for (a, b) in &links {
                forward.link(&id(*a), &id(*b));
            }
            let mut reordered = links.clone();
            shuffle(&mut reordered, seed);
            let mut backward = LinkGraph::new();
            for (a, b) in reordered.iter().rev() {
                backward.link(&id(*b), &id(*a));
            }
            prop_assert_eq!(forward.components(), backward.components());
        }
    }
}
