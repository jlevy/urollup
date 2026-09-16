//! Reading many sources on bounded workers, in file order (design §8.3).
//!
//! Adapted from ccusage `rust/adapters/common/src/lib.rs` at `bd7f89b`; see PROVENANCE.md
//! and THIRD-PARTY-NOTICES.md. Changes: the two `expect` panics are a
//! [`ParallelReadError`], the worker count is an explicit bound rather than
//! `available_parallelism` alone, sizes come from the caller instead of a `metadata` call
//! that silently treats an unreadable file as empty, and the results keep input order.
//!
//! Chunking by size rather than by a work queue means one very large file still pins one
//! worker, which is why the caller passes a worker bound. Results are returned in input
//! order, so everything downstream of the read is a deterministic merge.

use std::num::NonZeroUsize;
use std::thread;

/// A worker thread panicked, which no read should do.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a source-reading worker panicked")]
pub struct ParallelReadError;

/// The default worker bound: the machine's parallelism, or 1 when it is unknown.
pub fn default_workers() -> NonZeroUsize {
    thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

/// Splits indexes into `chunks` groups of roughly equal weight, largest first.
///
/// Equal weights keep input order, so the split is deterministic.
fn chunk_by_weight(weights: &[u64], chunks: usize) -> Vec<Vec<usize>> {
    let chunks = chunks.max(1);
    let mut ordered: Vec<(usize, u64)> = weights.iter().copied().enumerate().collect();
    ordered.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut groups = vec![Vec::new(); chunks];
    let mut loads = vec![0u64; chunks];
    for (index, weight) in ordered {
        let target = loads
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.cmp(b.1).then(a.0.cmp(&b.0)))
            .map_or(0, |(slot, _)| slot);
        if let (Some(group), Some(load)) = (groups.get_mut(target), loads.get_mut(target)) {
            group.push(index);
            *load = load.saturating_add(weight);
        }
    }
    groups.retain(|group| !group.is_empty());
    groups
}

/// Applies `read` to every item on at most `workers` threads and returns the results in
/// input order.
///
/// `weight` orders the work so that large items start first; a wrong weight costs speed,
/// never correctness.
pub fn read_in_parallel<T, U, W, R>(
    items: &[T],
    workers: NonZeroUsize,
    weight: W,
    read: R,
) -> Result<Vec<U>, ParallelReadError>
where
    T: Sync,
    U: Send,
    W: Fn(&T) -> u64,
    R: Fn(&T) -> U + Sync,
{
    let worker_count = workers.get().min(items.len());
    if worker_count <= 1 {
        return Ok(items.iter().map(&read).collect());
    }
    let weights: Vec<u64> = items.iter().map(weight).collect();
    let chunks = chunk_by_weight(&weights, worker_count);
    let read = &read;
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            handles.push(scope.spawn(move || {
                chunk
                    .into_iter()
                    .filter_map(|index| items.get(index).map(|item| (index, read(item))))
                    .collect::<Vec<_>>()
            }));
        }
        let mut results: Vec<Option<U>> = Vec::new();
        results.resize_with(items.len(), || None);
        for handle in handles {
            for (index, value) in handle.join().map_err(|_| ParallelReadError)? {
                if let Some(slot) = results.get_mut(index) {
                    *slot = Some(value);
                }
            }
        }
        results.into_iter().collect::<Option<Vec<U>>>().ok_or(ParallelReadError)
    })
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::{chunk_by_weight, read_in_parallel};

    fn workers(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).unwrap()
    }

    #[test]
    fn results_keep_input_order_on_every_worker_count() {
        let items: Vec<u64> = (0..64).collect();
        let expected: Vec<u64> = items.iter().map(|item| item * 2).collect();
        for count in [1, 2, 3, 8, 64, 128] {
            let read =
                read_in_parallel(&items, workers(count), |item| *item, |item| item * 2).unwrap();
            assert_eq!(read, expected, "worker count {count}");
        }
        assert!(
            read_in_parallel(&[] as &[u64], workers(4), |item| *item, |item| *item)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn chunks_balance_weight_and_stay_deterministic() {
        let weights = [100, 1, 1, 1, 100, 50];
        let chunks = chunk_by_weight(&weights, 3);
        assert_eq!(chunks, chunk_by_weight(&weights, 3));
        let mut covered: Vec<usize> = chunks.iter().flatten().copied().collect();
        covered.sort_unstable();
        assert_eq!(covered, vec![0, 1, 2, 3, 4, 5]);
        let loads: Vec<u64> =
            chunks.iter().map(|chunk| chunk.iter().map(|index| weights[*index]).sum()).collect();
        assert!(loads.iter().max().unwrap() - loads.iter().min().unwrap() <= 100);
    }

    #[test]
    fn a_zero_item_read_never_spawns() {
        let items = vec![7u64];
        assert_eq!(read_in_parallel(&items, workers(8), |_| 0, |item| *item).unwrap(), items);
    }
}
