//! Reading many sources on bounded workers, in file order (design §8.3).
//!
//! Adapted from ccusage `rust/adapters/common/src/lib.rs` at `bd7f89b`; see PROVENANCE.md
//! and THIRD-PARTY-NOTICES.md. Changes: the two `expect` panics are a
//! [`ParallelReadError`], the worker count is an explicit bound rather than
//! `available_parallelism` alone, capped at [`MAX_DEFAULT_WORKERS`] by default, a file's
//! size only orders the work ([`source_weight`]) instead of silently treating an
//! unreadable file as empty, workers pull items from one shared queue instead of fixed
//! chunks, a fallible read stops starting items after a failure, and the results keep
//! input order.
//!
//! Workers pull items heaviest first from one queue, so an underestimated weight, such as a
//! compressed file that expands more than expected, leaves the other workers free to take
//! the remaining items instead of idling behind a fixed chunk. One very large file still
//! pins one worker, which is why the caller passes a worker bound. Results are returned in
//! input order, so everything downstream of the read is a deterministic merge.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::sources::manifest::Representation;
use crate::sources::reader::LogicalSource;

/// The most workers [`default_workers`] chooses: past this, contention on the disk and
/// the allocator outweighs more decoding threads.
pub const MAX_DEFAULT_WORKERS: NonZeroUsize = match NonZeroUsize::new(8) {
    Some(workers) => workers,
    None => NonZeroUsize::MIN,
};

/// How many decoded bytes one compressed byte is assumed to expand to when ordering work.
const ZSTD_WEIGHT_EXPANSION: u64 = 8;

/// A worker thread panicked, which no read should do.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a source-reading worker panicked")]
pub struct ParallelReadError;

/// The default worker bound: the machine's parallelism, at most [`MAX_DEFAULT_WORKERS`],
/// or 1 when it is unknown.
pub fn default_workers() -> NonZeroUsize {
    thread::available_parallelism().unwrap_or(NonZeroUsize::MIN).min(MAX_DEFAULT_WORKERS)
}

/// The estimated decoding work of a logical source: its primary file's length, scaled
/// for compression, or 0 when the file cannot be inspected.
///
/// The weight only orders work, so a file that changes or vanishes before it is read
/// costs speed, never correctness.
pub fn source_weight(files: &LogicalSource) -> u64 {
    let Some((path, representation)) = files.primary() else { return 0 };
    let length = std::fs::metadata(path).map_or(0, |metadata| metadata.len());
    match representation {
        Representation::Plain => length,
        Representation::Zstd => length.saturating_mul(ZSTD_WEIGHT_EXPANSION),
    }
}

/// Item indexes heaviest first; equal weights keep input order, so the order is
/// deterministic.
fn heaviest_first(weights: &[u64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by(|a, b| weights.get(*b).cmp(&weights.get(*a)).then(a.cmp(b)));
    order
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
    run_in_parallel(items, workers, weight, |_, item| read(item))
}

/// Applies a fallible `read` to every item on at most `workers` threads and returns the
/// values in input order, or the error of the first failing item in input order.
///
/// That is the error a sequential read returns. Once an item has failed, no later item
/// is started, because its result could not change the outcome; earlier items still run,
/// since one of them may fail first.
pub fn try_read_in_parallel<T, U, E, W, R>(
    items: &[T],
    workers: NonZeroUsize,
    weight: W,
    read: R,
) -> Result<Vec<U>, E>
where
    T: Sync,
    U: Send,
    E: Send + From<ParallelReadError>,
    W: Fn(&T) -> u64,
    R: Fn(&T) -> Result<U, E> + Sync,
{
    let first_failure = AtomicUsize::new(usize::MAX);
    let results = run_in_parallel(items, workers, weight, |index, item| {
        if index > first_failure.load(Ordering::Relaxed) {
            return None;
        }
        let result = read(item);
        if result.is_err() {
            first_failure.fetch_min(index, Ordering::Relaxed);
        }
        Some(result)
    })?;
    let mut values = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Some(Ok(value)) => values.push(value),
            Some(Err(error)) => return Err(error),
            // An item is only skipped after an earlier item failed, and that failure was
            // returned above, so this is never reached; it fails rather than truncates.
            None => return Err(ParallelReadError.into()),
        }
    }
    Ok(values)
}

/// Applies `read` to every item and its input index on at most `workers` threads, and
/// returns the results in input order.
fn run_in_parallel<T, U, W, R>(
    items: &[T],
    workers: NonZeroUsize,
    weight: W,
    read: R,
) -> Result<Vec<U>, ParallelReadError>
where
    T: Sync,
    U: Send,
    W: Fn(&T) -> u64,
    R: Fn(usize, &T) -> U + Sync,
{
    let worker_count = workers.get().min(items.len());
    if worker_count <= 1 {
        return Ok(items.iter().enumerate().map(|(index, item)| read(index, item)).collect());
    }
    let weights: Vec<u64> = items.iter().map(weight).collect();
    let order = heaviest_first(&weights);
    let next = AtomicUsize::new(0);
    let (read, order, next) = (&read, &order, &next);
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            handles.push(scope.spawn(move || {
                let mut done = Vec::new();
                while let Some(&index) = order.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Some(item) = items.get(index) {
                        done.push((index, read(index, item)));
                    }
                }
                done
            }));
        }
        let mut results: Vec<Option<U>> = Vec::new();
        results.resize_with(items.len(), || None);
        let mut panicked = false;
        for handle in handles {
            match handle.join() {
                Ok(done) => {
                    for (index, value) in done {
                        if let Some(slot) = results.get_mut(index) {
                            *slot = Some(value);
                        }
                    }
                }
                Err(_) => panicked = true,
            }
        }
        if panicked {
            return Err(ParallelReadError);
        }
        results.into_iter().collect::<Option<Vec<U>>>().ok_or(ParallelReadError)
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::num::NonZeroUsize;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{
        MAX_DEFAULT_WORKERS, ParallelReadError, default_workers, heaviest_first, read_in_parallel,
        source_weight, try_read_in_parallel,
    };
    use crate::sources::reader::LogicalSource;

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
            let reversed =
                read_in_parallel(&items, workers(count), |item| 64 - *item, |item| item * 2)
                    .unwrap();
            assert_eq!(reversed, expected, "worker count {count}, reversed weights");
        }
        assert!(
            read_in_parallel(&[] as &[u64], workers(4), |item| *item, |item| *item)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn work_is_ordered_heaviest_first_and_stays_deterministic() {
        let weights = [100, 1, 1, 1, 100, 50];
        let order = heaviest_first(&weights);
        assert_eq!(order, vec![0, 4, 5, 1, 2, 3]);
        assert_eq!(order, heaviest_first(&weights));
    }

    #[test]
    fn a_zero_item_read_never_spawns() {
        let items = vec![7u64];
        assert_eq!(read_in_parallel(&items, workers(8), |_| 0, |item| *item).unwrap(), items);
    }

    #[test]
    fn every_item_is_read_exactly_once() {
        let items: Vec<usize> = (0..257).collect();
        let reads = AtomicUsize::new(0);
        let read = read_in_parallel(
            &items,
            workers(8),
            |item| u64::try_from(*item % 7).unwrap(),
            |item| {
                reads.fetch_add(1, Ordering::Relaxed);
                *item
            },
        )
        .unwrap();
        assert_eq!(read, items);
        assert_eq!(reads.load(Ordering::Relaxed), items.len());
    }

    #[derive(Debug, Eq, PartialEq)]
    enum Failure {
        Item(usize),
        Worker,
    }

    impl From<ParallelReadError> for Failure {
        fn from(_: ParallelReadError) -> Self {
            Self::Worker
        }
    }

    #[test]
    fn the_first_failure_in_input_order_wins_on_every_worker_count() {
        let items: Vec<usize> = (0..64).collect();
        for count in [1, 2, 3, 8, 64] {
            // Later items are heavier, so they start first and fail before item 10 does.
            let result = try_read_in_parallel(
                &items,
                workers(count),
                |item| u64::try_from(*item).unwrap(),
                |item| {
                    if *item % 10 == 0 && *item > 0 { Err(Failure::Item(*item)) } else { Ok(*item) }
                },
            );
            assert_eq!(result, Err(Failure::Item(10)), "worker count {count}");
            let values = try_read_in_parallel(
                &items,
                workers(count),
                |_| 0,
                |item| Ok::<_, Failure>(*item + 1),
            )
            .unwrap();
            assert_eq!(values, (1..=64).collect::<Vec<_>>(), "worker count {count}");
        }
    }

    #[test]
    fn a_sequential_fallible_read_stops_at_the_first_failure() {
        let items: Vec<usize> = (0..16).collect();
        let reads = AtomicUsize::new(0);
        let result = try_read_in_parallel(
            &items,
            workers(1),
            |_| 0,
            |item| {
                reads.fetch_add(1, Ordering::Relaxed);
                if *item == 3 { Err(Failure::Item(3)) } else { Ok(*item) }
            },
        );
        assert_eq!(result, Err(Failure::Item(3)));
        assert_eq!(reads.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn a_panicking_worker_is_an_error() {
        let items: Vec<usize> = (0..8).collect();
        let result = read_in_parallel(
            &items,
            workers(4),
            |_| 0,
            |item| {
                assert_ne!(*item, 5, "synthetic worker panic");
                *item
            },
        );
        assert_eq!(result, Err(ParallelReadError));
    }

    #[test]
    fn the_default_worker_bound_is_capped() {
        assert!(default_workers() <= MAX_DEFAULT_WORKERS);
        assert_eq!(MAX_DEFAULT_WORKERS.get(), 8);
    }

    #[test]
    fn source_weights_scale_compressed_files_and_tolerate_missing_ones() {
        let root = tempfile::tempdir().unwrap();
        let plain = root.path().join("a.jsonl");
        let compressed = root.path().join("b.jsonl.zst");
        fs::write(&plain, [b'x'; 10]).unwrap();
        fs::write(&compressed, [b'x'; 10]).unwrap();

        let weight = |plain: Option<&std::path::Path>, compressed: Option<&std::path::Path>| {
            source_weight(&LogicalSource {
                plain: plain.map(std::path::Path::to_owned),
                compressed: compressed.map(std::path::Path::to_owned),
            })
        };
        assert_eq!(weight(Some(&plain), None), 10);
        assert_eq!(weight(None, Some(&compressed)), 80);
        assert_eq!(weight(Some(&root.path().join("missing.jsonl")), None), 0);
        assert_eq!(weight(None, None), 0);
    }
}
