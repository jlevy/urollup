//! Cumulative counters to per-request deltas within verified counter epochs (design §3.3,
//! §3.4).
//!
//! Several sources write running totals rather than per-request usage: legacy Codex
//! `token_count` events, app-server thread totals and `codex exec` `turn.completed.usage`.
//! One rule serves all of them, so they cannot disagree:
//!
//! - A total identical to the previous one adds nothing, as does a total that omits
//!   categories but changes none it reports.
//! - A total with no category below the previous one advances the epoch; its delta is the
//!   category-wise difference.
//! - A total with any category below the previous one opens a new counter epoch with a
//!   [`CounterEvent::Reset`] diagnostic, and its delta is the new total itself.
//! - A sequence number that skips values is a [`CounterEvent::Gap`] diagnostic; the delta
//!   still covers the skipped updates, since the total includes them.
//! - A forked or resumed child continues its parent's running total, so the tracker starts
//!   from the inherited total and the child's first delta excludes it.
//!
//! Within an epoch, a category a total omits keeps its last value. A reset discards
//! the previous epoch's baseline, including omitted categories. A category reported for
//! the first time in an epoch counts from zero. All arithmetic is checked.

#![deny(clippy::arithmetic_side_effects)]

use super::tokens::{TokenMeasures, TokenOverflow};

/// What one running-total observation did.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CounterEvent {
    /// The total equals the previous one; nothing is added.
    Repeated,
    /// The total advanced within the current epoch.
    Advanced,
    /// A category decreased, so a new epoch starts at this total.
    Reset,
    /// The sequence skipped values before this total, which still advanced.
    Gap {
        /// The sequence number expected next.
        expected: u64,
        /// The sequence number observed.
        observed: u64,
    },
}

/// The delta one running total contributes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterStep {
    /// Usage newly consumed since the previous total; all `None` when repeated.
    pub delta: TokenMeasures,
    /// What happened.
    pub event: CounterEvent,
    /// The epoch this total belongs to, counting from 0.
    pub epoch: u32,
}

/// Turns a stream of running totals for one thread into per-update deltas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunningTotal {
    last: TokenMeasures,
    last_sequence: Option<u64>,
    epoch: u32,
}

impl RunningTotal {
    /// A tracker for a thread that starts from zero.
    pub fn new() -> Self {
        Self::inheriting(TokenMeasures::default())
    }

    /// A tracker for a forked or resumed child that continues `inherited`, the parent's
    /// running total at the fork, which the child's deltas exclude.
    pub fn inheriting(inherited: TokenMeasures) -> Self {
        Self { last: inherited, last_sequence: None, epoch: 0 }
    }

    /// Records the next running total, optionally with its native sequence number, and
    /// returns its delta.
    pub fn observe(
        &mut self,
        total: &TokenMeasures,
        sequence: Option<u64>,
    ) -> Result<CounterStep, TokenOverflow> {
        let gap = match (self.last_sequence, sequence) {
            (Some(previous), Some(current)) => {
                let expected =
                    previous.checked_add(1).ok_or(TokenOverflow { category: "sequence" })?;
                (current > expected).then_some(CounterEvent::Gap { expected, observed: current })
            }
            (None | Some(_), None) | (None, Some(_)) => None,
        };
        if sequence.is_some() {
            self.last_sequence = sequence;
        }

        let previous = self.last.categories();
        let current = total.categories();
        let decreased = previous
            .iter()
            .zip(&current)
            .any(|((_, before), (_, now))| matches!((before, now), (Some(b), Some(n)) if n < b));

        let (delta, event) = if decreased {
            self.epoch = self.epoch.checked_add(1).ok_or(TokenOverflow { category: "epoch" })?;
            (*total, CounterEvent::Reset)
        } else if merged(&self.last, total) == self.last {
            (TokenMeasures::default(), CounterEvent::Repeated)
        } else {
            (difference(&self.last, total)?, gap.unwrap_or(CounterEvent::Advanced))
        };
        self.last = if decreased { *total } else { merged(&self.last, total) };
        Ok(CounterStep { delta, event, epoch: self.epoch })
    }
}

impl Default for RunningTotal {
    fn default() -> Self {
        Self::new()
    }
}

/// The later total, keeping earlier values for categories it does not report.
fn merged(previous: &TokenMeasures, current: &TokenMeasures) -> TokenMeasures {
    let never = previous
        .zip_with(current, |_, before, now| Ok::<_, std::convert::Infallible>(now.or(before)));
    match never {
        Ok(measures) => measures,
        Err(infallible) => match infallible {},
    }
}

/// Category-wise `current - previous` for categories `current` reports; the caller has
/// already established that no reported category decreased.
fn difference(
    previous: &TokenMeasures,
    current: &TokenMeasures,
) -> Result<TokenMeasures, TokenOverflow> {
    previous.zip_with(current, |category, before, now| match now {
        None => Ok(None),
        Some(now) => {
            now.checked_sub(before.unwrap_or(0)).map(Some).ok_or(TokenOverflow { category })
        }
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{CounterEvent, RunningTotal};
    use crate::ledger::tokens::TokenMeasures;

    fn io(input: u64, output: u64) -> TokenMeasures {
        TokenMeasures {
            uncached_input: Some(input),
            output: Some(output),
            ..TokenMeasures::default()
        }
    }

    #[test]
    fn identical_consecutive_totals_add_nothing() {
        let mut tracker = RunningTotal::new();
        assert_eq!(tracker.observe(&io(10, 2), None).unwrap().delta, io(10, 2));
        let repeat = tracker.observe(&io(10, 2), None).unwrap();
        assert_eq!(repeat.event, CounterEvent::Repeated);
        assert!(repeat.delta.is_unreported());
        let next = tracker.observe(&io(15, 3), None).unwrap();
        assert_eq!((next.event, next.delta), (CounterEvent::Advanced, io(5, 1)));
    }

    #[test]
    fn a_decrease_in_any_category_opens_a_new_epoch() {
        let mut tracker = RunningTotal::new();
        tracker.observe(&io(100, 10), None).unwrap();
        let reset = tracker.observe(&io(120, 4), None).unwrap();
        assert_eq!((reset.event, reset.epoch, reset.delta), (CounterEvent::Reset, 1, io(120, 4)));
        let after = tracker.observe(&io(130, 5), None).unwrap();
        assert_eq!((after.event, after.epoch, after.delta), (CounterEvent::Advanced, 1, io(10, 1)));
    }

    #[test]
    fn a_reset_discards_omitted_categories_from_the_previous_epoch() {
        let mut tracker = RunningTotal::new();
        tracker.observe(&io(100, 100), None).unwrap();
        let partial = TokenMeasures { uncached_input: Some(5), ..TokenMeasures::default() };
        let reset = tracker.observe(&partial, None).unwrap();
        assert_eq!((reset.event, reset.epoch, reset.delta), (CounterEvent::Reset, 1, partial));
        let next = tracker.observe(&io(10, 1), None).unwrap();
        assert_eq!((next.event, next.epoch, next.delta), (CounterEvent::Advanced, 1, io(5, 1)));
        let partial = TokenMeasures { uncached_input: Some(15), ..TokenMeasures::default() };
        tracker.observe(&partial, None).unwrap();
        let next = tracker.observe(&io(20, 2), None).unwrap();
        assert_eq!((next.event, next.epoch, next.delta), (CounterEvent::Advanced, 1, io(5, 1)));
    }

    #[test]
    fn a_forked_child_excludes_its_inherited_total() {
        let mut child = RunningTotal::inheriting(io(500, 50));
        let first = child.observe(&io(530, 60), None).unwrap();
        assert_eq!((first.event, first.delta), (CounterEvent::Advanced, io(30, 10)));
    }

    #[test]
    fn skipped_sequence_numbers_are_a_gap_that_still_counts() {
        let mut tracker = RunningTotal::new();
        tracker.observe(&io(1, 1), Some(4)).unwrap();
        let step = tracker.observe(&io(9, 3), Some(7)).unwrap();
        assert_eq!(step.event, CounterEvent::Gap { expected: 5, observed: 7 });
        assert_eq!(step.delta, io(8, 2));
    }

    proptest! {
        #[test]
        fn deltas_within_one_epoch_sum_to_the_advance(
            increments in prop::collection::vec((0u64..1_000, 0u64..1_000), 1..30),
            base in (0u64..10_000, 0u64..10_000),
        ) {
            let inherited = io(base.0, base.1);
            let mut tracker = RunningTotal::inheriting(inherited);
            let mut total = inherited;
            let mut summed = TokenMeasures::default();
            for (input, output) in increments {
                total = total.checked_add(&io(input, output)).unwrap();
                let step = tracker.observe(&total, None).unwrap();
                prop_assert_ne!(step.event, CounterEvent::Reset);
                summed = summed.checked_add(&step.delta).unwrap();
            }
            let advance = (
                total.uncached_input.unwrap().checked_sub(base.0).unwrap(),
                total.output.unwrap().checked_sub(base.1).unwrap(),
            );
            prop_assert_eq!(summed.uncached_input.unwrap_or(0), advance.0);
            prop_assert_eq!(summed.output.unwrap_or(0), advance.1);
        }
    }
}
