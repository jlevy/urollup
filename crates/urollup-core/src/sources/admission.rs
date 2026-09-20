//! Shared admission for retained request-bearing rows, before normalization.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::ledger::reconcile::ReconcileError;

/// One budget shared by all sources of an adapter invocation. Reservations live until
/// that invocation ends: completed worker results still retain their rows.
pub(crate) struct Admission {
    retained: AtomicUsize,
    stopped: AtomicBool,
    maximum: usize,
}

impl Admission {
    pub(crate) fn new(maximum: usize) -> Self {
        Self { retained: AtomicUsize::new(0), stopped: AtomicBool::new(false), maximum }
    }

    pub(crate) fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    /// Reserve before retaining a request-bearing record, including pending copies.
    /// Completed workers do not release slots. Once refused, every worker stops at its
    /// next visited record. No more than `maximum` such rows can be admitted.
    pub(crate) fn reserve(&self) -> bool {
        if self.stopped() {
            return false;
        }
        if self
            .retained
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count < self.maximum).then(|| count + 1)
            })
            .is_err()
        {
            self.stopped.store(true, Ordering::Relaxed);
            return false;
        }
        true
    }

    pub(crate) fn error(&self) -> ReconcileError {
        // We stopped early: report the first refused slot, not a whole-input count.
        ReconcileError::CapacityExceeded {
            observations: self.maximum.saturating_add(1),
            maximum: self.maximum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_reservations_never_exceed_the_shared_limit() {
        let admission = Admission::new(97);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let admission = &admission;
                scope.spawn(move || while admission.reserve() {});
            }
        });
        assert_eq!(admission.retained.load(Ordering::Relaxed), 97);
        assert!(admission.stopped());
        assert!(!admission.reserve());
        assert!(!Admission::new(0).reserve());
    }
}
