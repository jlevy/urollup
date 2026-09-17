//! Short lists stored inline.

/// A list whose first `N` items are stored inline, spilling to the heap beyond them.
///
/// Observations carry one or two keys and at most one invariant. A `Vec` allocates a
/// minimum capacity of four for each, which on a whole-history run is hundreds of thousands
/// of small allocations. Items fill the inline slots in order, so the derived comparison,
/// equality and hashing agree with comparing the item sequences.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InlineList<T, const N: usize> {
    inline: [Option<T>; N],
    #[expect(
        clippy::box_collection,
        reason = "a boxed vector keeps the rarely used spill slot at 8 bytes instead of 24"
    )]
    spilled: Option<Box<Vec<T>>>,
}

impl<T, const N: usize> Default for InlineList<T, N> {
    fn default() -> Self {
        Self { inline: std::array::from_fn(|_| None), spilled: None }
    }
}

impl<T, const N: usize> InlineList<T, N> {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an item.
    pub fn push(&mut self, item: T) {
        match self.inline.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => *slot = Some(item),
            None => self.spilled.get_or_insert_with(Box::default).push(item),
        }
    }

    /// The number of items.
    pub fn len(&self) -> usize {
        self.inline.iter().flatten().count()
            + self.spilled.as_ref().map_or(0, |spilled| spilled.len())
    }

    /// Whether the list has no items.
    pub fn is_empty(&self) -> bool {
        self.first().is_none()
    }

    /// The first item.
    pub fn first(&self) -> Option<&T> {
        self.iter().next()
    }

    /// The items in order.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            inline: self.inline.iter(),
            spilled: self.spilled.as_deref().map_or(&[][..], Vec::as_slice).iter(),
        }
    }
}

impl<T: Ord, const N: usize> InlineList<T, N> {
    /// Sorts the items and removes repeated ones.
    pub fn sort_dedup(&mut self) {
        if self.spilled.is_none() {
            let count = self.inline.iter().take_while(|slot| slot.is_some()).count();
            let items = &mut self.inline[..count];
            items.sort();
            let mut kept = 0;
            for read in 0..count {
                let item = items[read].take();
                if kept > 0 && items[kept - 1] == item {
                    continue;
                }
                items[kept] = item;
                kept += 1;
            }
            return;
        }
        let mut items: Vec<T> = self.inline.iter_mut().filter_map(Option::take).collect();
        items.extend(self.spilled.take().into_iter().flat_map(|spilled| *spilled));
        items.sort();
        items.dedup();
        *self = items.into_iter().collect();
    }
}

impl<T, const N: usize> FromIterator<T> for InlineList<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut list = Self::default();
        for item in items {
            list.push(item);
        }
        list
    }
}

impl<T, const N: usize> From<Vec<T>> for InlineList<T, N> {
    fn from(items: Vec<T>) -> Self {
        items.into_iter().collect()
    }
}

/// An iterator over an [`InlineList`]'s items.
pub struct Iter<'a, T> {
    inline: std::slice::Iter<'a, Option<T>>,
    spilled: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.inline.by_ref().flatten().next().or_else(|| self.spilled.next())
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a InlineList<T, N> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::InlineList;

    proptest! {
        #[test]
        fn an_inline_list_behaves_like_a_vector(
            left in prop::collection::vec(0u8..6, 0..6),
            right in prop::collection::vec(0u8..6, 0..6),
        ) {
            let list: InlineList<u8, 2> = left.clone().into();
            let other: InlineList<u8, 2> = right.clone().into();
            prop_assert_eq!(list.iter().copied().collect::<Vec<_>>(), left.clone());
            prop_assert_eq!(list.len(), left.len());
            prop_assert_eq!(list.is_empty(), left.is_empty());
            prop_assert_eq!(list.first(), left.first());
            prop_assert_eq!(list.cmp(&other), left.cmp(&right));
            prop_assert_eq!(list == other, left == right);

            let mut sorted = list;
            sorted.sort_dedup();
            let mut expected = left;
            expected.sort_unstable();
            expected.dedup();
            prop_assert_eq!(sorted.iter().copied().collect::<Vec<_>>(), expected.clone());
            prop_assert_eq!(sorted, InlineList::<u8, 2>::from(expected));
        }
    }
}
