//! A vector stored in fixed-capacity chunks.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Index, IndexMut};

/// A vector whose items live in separately allocated chunks of `2^SHIFT` items.
///
/// Reconciliation turns hundreds of thousands of wide observation rows into request rows.
/// On macOS, memory a process frees keeps counting toward its physical footprint until the
/// allocator reuses it, and measurements showed that even freed blocks of 128 MiB are not
/// given back, while a later allocation of the same or a smaller size reuses a freed block
/// without growing the footprint. So consuming observations lowers the peak only when the
/// requests built meanwhile land in the blocks the observations free.
///
/// Chunks make that possible where one contiguous vector cannot: that vector shrinks only
/// by copying, and a table built beside it needs one fresh region. Here
/// [`truncate`](Self::truncate) frees whole chunks, and a chunked table of rows no wider
/// than the freed ones allocates chunks that fit in them.
///
/// Every chunk but the last is full, so an index finds its chunk by a shift and the layout
/// depends only on the length. The first chunk grows geometrically, so a short list stays
/// small; later chunks are allocated at full capacity. The default of 16,384 items keeps a
/// chunk of 288-byte observation rows under 5 MiB.
///
/// Reordering takes a `u32` permutation, so a sorted or permuted list holds fewer than
/// 2^32 items.
#[derive(Clone)]
pub struct ChunkedVec<T, const SHIFT: u32 = 14> {
    chunks: Vec<Vec<T>>,
    len: usize,
}

/// An iterator over a [`ChunkedVec`]'s items.
pub type Iter<'a, T> = std::iter::Flatten<std::slice::Iter<'a, Vec<T>>>;

/// An iterator over a [`ChunkedVec`]'s items, mutably.
pub type IterMut<'a, T> = std::iter::Flatten<std::slice::IterMut<'a, Vec<T>>>;

/// The capacity of a first chunk, which then doubles until it is full.
const FIRST_CAPACITY: usize = 64;

impl<T, const SHIFT: u32> Default for ChunkedVec<T, SHIFT> {
    fn default() -> Self {
        Self { chunks: Vec::new(), len: 0 }
    }
}

impl<T, const SHIFT: u32> ChunkedVec<T, SHIFT> {
    /// Items per chunk.
    const CHUNK: usize = {
        assert!(SHIFT < 32, "a chunk holds fewer than 2^32 items");
        1 << SHIFT
    };

    /// An empty list, which allocates nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of items.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the list has no items.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends an item.
    pub fn push(&mut self, item: T) {
        let full = self.len & (Self::CHUNK - 1) == 0;
        match self.chunks.last_mut() {
            Some(chunk) if !full => {
                if chunk.len() == chunk.capacity() {
                    // Only a first chunk starts below full capacity.
                    chunk.reserve_exact(chunk.len().min(Self::CHUNK - chunk.len()));
                }
                chunk.push(item);
            }
            _ => {
                let capacity = if self.chunks.is_empty() {
                    FIRST_CAPACITY.min(Self::CHUNK)
                } else {
                    Self::CHUNK
                };
                let mut chunk = Vec::with_capacity(capacity);
                chunk.push(item);
                self.chunks.push(chunk);
            }
        }
        self.len += 1;
    }

    /// The item at `index`.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.chunks.get(index >> SHIFT)?.get(index & (Self::CHUNK - 1))
    }

    /// The item at `index`, mutably.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.chunks.get_mut(index >> SHIFT)?.get_mut(index & (Self::CHUNK - 1))
    }

    /// Swaps two items, which may lie in different chunks.
    ///
    /// # Panics
    ///
    /// Panics when either index is out of bounds, as [`slice::swap`] does.
    pub fn swap(&mut self, a: usize, b: usize) {
        let (chunk_a, slot_a) = (a >> SHIFT, a & (Self::CHUNK - 1));
        let (chunk_b, slot_b) = (b >> SHIFT, b & (Self::CHUNK - 1));
        match chunk_a.cmp(&chunk_b) {
            Ordering::Equal => self.chunks[chunk_a].swap(slot_a, slot_b),
            Ordering::Less => {
                let (low, high) = self.chunks.split_at_mut(chunk_b);
                std::mem::swap(&mut low[chunk_a][slot_a], &mut high[0][slot_b]);
            }
            Ordering::Greater => {
                let (low, high) = self.chunks.split_at_mut(chunk_a);
                std::mem::swap(&mut low[chunk_b][slot_b], &mut high[0][slot_a]);
            }
        }
    }

    /// Shortens the list to `len` items, dropping the rest and freeing every chunk that no
    /// longer holds an item. A longer `len` does nothing.
    pub fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        let chunks = len.div_ceil(Self::CHUNK);
        self.chunks.truncate(chunks);
        if let Some(last) = self.chunks.last_mut() {
            last.truncate(len - (chunks - 1) * Self::CHUNK);
        }
        self.len = len;
    }

    /// The items in order.
    pub fn iter(&self) -> Iter<'_, T> {
        self.chunks.iter().flatten()
    }

    /// The items in order, mutably.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.chunks.iter_mut().flatten()
    }

    /// Binary searches items sorted by `compare`, as [`slice::binary_search_by`] does: the
    /// index of a matching item, or the index where one could be inserted.
    pub fn binary_search_by(&self, mut compare: impl FnMut(&T) -> Ordering) -> Result<usize, usize> {
        // The first chunk whose last item is not below the target holds it or its place.
        let chunk = self
            .chunks
            .partition_point(|items| items.last().is_some_and(|last| compare(last) == Ordering::Less));
        let Some(items) = self.chunks.get(chunk) else {
            return Err(self.len);
        };
        let base = chunk << SHIFT;
        items.binary_search_by(compare).map(|slot| base + slot).map_err(|slot| base + slot)
    }

    /// Reorders the items so position `k` holds the item that was at `order[k]`, by swaps;
    /// `order` must be a permutation of `0..len` and is left as the identity.
    pub fn permute(&mut self, order: &mut [u32]) {
        debug_assert_eq!(order.len(), self.len, "a permutation covers every item");
        apply_permutation(order, |a, b| self.swap(a, b));
    }

    /// Sorts the items with `compare` without moving them during the sort: a permutation of
    /// indices is sorted and then applied by swaps, so no buffer of items is allocated. Like
    /// [`slice::sort_unstable_by`], equal items may be reordered.
    pub fn sort_unstable_by(&mut self, mut compare: impl FnMut(&T, &T) -> Ordering) {
        let count = u32::try_from(self.len).expect("a sorted list holds fewer than 2^32 items");
        let mut order: Vec<u32> = (0..count).collect();
        order
            .sort_unstable_by(|&left, &right| compare(&self[left as usize], &self[right as usize]));
        self.permute(&mut order);
    }

    /// Keeps only the items `keep` accepts, visiting each once in order.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        let mut kept = 0;
        for read in 0..self.len {
            if keep(&self[read]) {
                if read != kept {
                    self.swap(read, kept);
                }
                kept += 1;
            }
        }
        self.truncate(kept);
    }

    /// Removes each item that `same` matches with the last item kept before it, as
    /// [`Vec::dedup_by`] does; `same` receives the later item first.
    pub fn dedup_by(&mut self, mut same: impl FnMut(&T, &T) -> bool) {
        let mut kept = 0;
        for read in 0..self.len {
            if kept > 0 && same(&self[read], &self[kept - 1]) {
                continue;
            }
            if read != kept {
                self.swap(read, kept);
            }
            kept += 1;
        }
        self.truncate(kept);
    }
}

impl<T: Ord, const SHIFT: u32> ChunkedVec<T, SHIFT> {
    /// Sorts the items; see [`sort_unstable_by`](Self::sort_unstable_by).
    pub fn sort_unstable(&mut self) {
        self.sort_unstable_by(Ord::cmp);
    }
}

impl<T: PartialEq, const SHIFT: u32> ChunkedVec<T, SHIFT> {
    /// Removes consecutive repeated items, keeping the first of each run.
    pub fn dedup(&mut self) {
        self.dedup_by(|later, earlier| later == earlier);
    }
}

/// Reorders a sequence so position `k` holds the item that was at `order[k]`, by calling
/// `swap` along the permutation's cycles; `order` is left as the identity.
fn apply_permutation(order: &mut [u32], mut swap: impl FnMut(usize, usize)) {
    for start in 0..order.len() {
        let mut current = start;
        loop {
            let source = order[current] as usize;
            if source == current {
                break;
            }
            order[current] = u32::try_from(current).expect("a permutation of u32 positions");
            if source == start {
                break;
            }
            swap(current, source);
            current = source;
        }
    }
}

impl<T, const SHIFT: u32> Index<usize> for ChunkedVec<T, SHIFT> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.chunks[index >> SHIFT][index & (Self::CHUNK - 1)]
    }
}

impl<T, const SHIFT: u32> IndexMut<usize> for ChunkedVec<T, SHIFT> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.chunks[index >> SHIFT][index & (Self::CHUNK - 1)]
    }
}

impl<T, const SHIFT: u32> FromIterator<T> for ChunkedVec<T, SHIFT> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut list = Self::new();
        for item in items {
            list.push(item);
        }
        list
    }
}

impl<T, const SHIFT: u32> From<Vec<T>> for ChunkedVec<T, SHIFT> {
    fn from(items: Vec<T>) -> Self {
        if items.is_empty() {
            return Self::new();
        }
        if items.len() <= Self::CHUNK {
            // A vector that fits in one chunk becomes that chunk without a copy.
            return Self { len: items.len(), chunks: vec![items] };
        }
        items.into_iter().collect()
    }
}

impl<'a, T, const SHIFT: u32> IntoIterator for &'a ChunkedVec<T, SHIFT> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, const SHIFT: u32> IntoIterator for &'a mut ChunkedVec<T, SHIFT> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: PartialEq, const SHIFT: u32> PartialEq for ChunkedVec<T, SHIFT> {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}

impl<T: Eq, const SHIFT: u32> Eq for ChunkedVec<T, SHIFT> {}

impl<T: fmt::Debug, const SHIFT: u32> fmt::Debug for ChunkedVec<T, SHIFT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{ChunkedVec, apply_permutation};

    /// Chunks of four items, so short lists cross chunk boundaries.
    type Small = ChunkedVec<(u8, usize), 2>;

    fn tagged(values: &[u8]) -> Vec<(u8, usize)> {
        values.iter().copied().zip(0..).collect()
    }

    fn chunk_lengths<T, const SHIFT: u32>(list: &ChunkedVec<T, SHIFT>) -> Vec<usize> {
        list.chunks.iter().map(Vec::len).collect()
    }

    proptest! {
        #[test]
        fn a_chunked_vector_behaves_like_a_vector(
            values in prop::collection::vec(0u8..6, 0..40),
            swaps in prop::collection::vec((0usize..40, 0usize..40), 0..20),
            cut in 0usize..48,
            pushed in prop::collection::vec(0u8..6, 0..10),
        ) {
            let mut expected = tagged(&values);
            let mut list: Small = expected.clone().into();
            prop_assert_eq!(list.len(), expected.len());
            prop_assert_eq!(list.is_empty(), expected.is_empty());
            prop_assert_eq!(list.iter().copied().collect::<Vec<_>>(), expected.clone());
            prop_assert_eq!(&list, &expected.iter().copied().collect::<Small>());

            for (a, b) in swaps {
                if a < expected.len() && b < expected.len() {
                    expected.swap(a, b);
                    list.swap(a, b);
                }
            }
            for index in 0..expected.len() + 2 {
                prop_assert_eq!(list.get(index), expected.get(index));
            }

            expected.truncate(cut);
            list.truncate(cut);
            for (offset, value) in pushed.into_iter().enumerate() {
                expected.push((value, 100 + offset));
                list.push((value, 100 + offset));
            }
            if let Some(first) = expected.first_mut() {
                first.1 += 1000;
                list[0].1 += 1000;
                *list.get_mut(0).unwrap() = *first;
            }
            prop_assert_eq!(list.iter().copied().collect::<Vec<_>>(), expected.clone());
            // Every chunk but the last is full, so the layout follows the length alone.
            let lengths = chunk_lengths(&list);
            prop_assert_eq!(lengths.len(), expected.len().div_ceil(4));
            prop_assert!(lengths.iter().rev().skip(1).all(|length| *length == 4));
        }

        #[test]
        fn sorting_deduplicating_and_retaining_match_a_vector(
            values in prop::collection::vec(0u8..6, 0..40),
            threshold in 0u8..6,
        ) {
            let mut expected = tagged(&values);
            let mut list: Small = expected.clone().into();
            // Sorting by value alone: equal values may be reordered, so compare values and
            // then the multiset of tags.
            expected.sort_unstable_by_key(|item| item.0);
            list.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            let sorted: Vec<(u8, usize)> = list.iter().copied().collect();
            prop_assert_eq!(
                sorted.iter().map(|item| item.0).collect::<Vec<_>>(),
                expected.iter().map(|item| item.0).collect::<Vec<_>>()
            );
            let mut tags: Vec<usize> = sorted.iter().map(|item| item.1).collect();
            tags.sort_unstable();
            prop_assert_eq!(tags, (0..values.len()).collect::<Vec<_>>());

            let mut plain: ChunkedVec<u8, 2> = values.clone().into();
            let mut plain_expected = values.clone();
            plain.sort_unstable();
            plain_expected.sort_unstable();
            prop_assert_eq!(plain.iter().copied().collect::<Vec<_>>(), plain_expected.clone());
            plain.dedup();
            plain_expected.dedup();
            prop_assert_eq!(plain.iter().copied().collect::<Vec<_>>(), plain_expected);

            // Unsorted input keeps the first of each run of repeats, like `Vec::dedup`.
            let mut runs: ChunkedVec<u8, 2> = values.clone().into();
            let mut runs_expected = values.clone();
            runs.dedup();
            runs_expected.dedup();
            prop_assert_eq!(runs.iter().copied().collect::<Vec<_>>(), runs_expected);

            let mut retained: Small = tagged(&values).into();
            let mut retained_expected = tagged(&values);
            let mut visited = Vec::new();
            retained.retain(|item| {
                visited.push(item.1);
                item.0 >= threshold
            });
            retained_expected.retain(|item| item.0 >= threshold);
            prop_assert_eq!(retained.iter().copied().collect::<Vec<_>>(), retained_expected);
            prop_assert_eq!(visited, (0..values.len()).collect::<Vec<_>>());
            prop_assert_eq!(retained.len(), retained.iter().count());
        }

        #[test]
        fn applying_a_sorting_permutation_matches_sorting(
            rows in prop::collection::vec(0u16..50, 0..64),
        ) {
            let mut order: Vec<u32> = (0..u32::try_from(rows.len()).unwrap()).collect();
            order.sort_by_key(|&index| (rows[index as usize], std::cmp::Reverse(index)));
            let expected: Vec<(u16, usize)> =
                order.iter().map(|&index| (rows[index as usize], index as usize)).collect();

            let mut tagged: Vec<(u16, usize)> = rows.iter().copied().zip(0..).collect();
            let mut vector_order = order.clone();
            apply_permutation(&mut vector_order, |a, b| tagged.swap(a, b));
            prop_assert_eq!(&tagged, &expected);
            prop_assert!(vector_order.iter().enumerate().all(|(position, value)| position == *value as usize));

            let mut chunked: ChunkedVec<(u16, usize), 3> =
                rows.iter().copied().zip(0..).collect();
            chunked.permute(&mut order);
            prop_assert_eq!(chunked.iter().copied().collect::<Vec<_>>(), expected);
            prop_assert!(order.iter().enumerate().all(|(position, value)| position == *value as usize));
        }
    }

    #[test]
    fn truncation_frees_whole_chunks_and_growth_fills_them_in_order() {
        let mut list: ChunkedVec<u32, 2> = (0..10).collect();
        assert_eq!(chunk_lengths(&list), [4, 4, 2]);
        list.truncate(8);
        assert_eq!(chunk_lengths(&list), [4, 4]);
        list.truncate(5);
        assert_eq!(chunk_lengths(&list), [4, 1]);
        list.truncate(4);
        assert_eq!(chunk_lengths(&list), [4]);
        list.push(40);
        assert_eq!(chunk_lengths(&list), [4, 1]);
        list.truncate(0);
        assert!(list.chunks.is_empty() && list.is_empty());
        list.push(7);
        assert_eq!((list.len(), list[0]), (1, 7));

        // Chunks after the first are allocated whole and never reallocate.
        let mut wide: ChunkedVec<u8, 8> = (0..=255).collect();
        assert_eq!(wide.chunks[0].capacity(), 256);
        wide.push(0);
        assert_eq!(wide.chunks[1].capacity(), 256);
        let before = wide.chunks[1].as_ptr();
        wide.extend_for_test(255);
        assert_eq!(wide.chunks[1].as_ptr(), before);
        assert_eq!(chunk_lengths(&wide), [256, 256]);
    }

    impl<T: Default, const SHIFT: u32> ChunkedVec<T, SHIFT> {
        fn extend_for_test(&mut self, count: usize) {
            for _ in 0..count {
                self.push(T::default());
            }
        }
    }
}
