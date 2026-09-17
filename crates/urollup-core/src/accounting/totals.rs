//! Ownership-aware totals and selections (design §4.2).
//!
//! - **Grand totals** count every counted request once, whatever its ownership, so they
//!   never depend on ownership resolution. They are also split by status (owned,
//!   ambiguous, unknown), and those three always sum to the grand total.
//! - **Unresolved** usage, from non-counted candidate-set members, is reported beside the
//!   totals and never added. Copy-only requests count as neither.
//! - **Selections** of threads count their owned requests and the ambiguous requests whose
//!   candidates all lie inside the selection. Ambiguous requests with only some candidates
//!   inside form the separate **possible** measure, which is never added. Unknown-owner
//!   requests belong to no selection.
//! - **Completeness** is partial whenever a counted member has no usage, unresolved or
//!   possible usage exists, or an unobserved coverage gap applies: a group with any unknown
//!   member is never complete.
//!
//! All arithmetic is checked; `clippy::arithmetic_side_effects` is denied in this module.

#![deny(clippy::arithmetic_side_effects)]

use std::collections::BTreeSet;

use crate::ledger::entities::{Counting, Ownership, Request};
use crate::ledger::identity::AnalyticalId;
use crate::ledger::reconcile::Ledger;
use crate::ledger::tokens::{TokenMeasures, TokenOverflow};

/// Request counts and token sums for one set of requests.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct UsageTotals {
    /// Requests in the set.
    pub requests: u64,
    /// Requests in the set with no usage revision, whose usage is unknown.
    pub requests_without_usage: u64,
    /// Category-wise sums of the selected usage revisions.
    pub tokens: TokenMeasures,
}

impl UsageTotals {
    fn add(&mut self, request: &Request) -> Result<(), TokenOverflow> {
        self.requests =
            self.requests.checked_add(1).ok_or(TokenOverflow { category: "requests" })?;
        match &request.usage {
            Some(selected) => {
                self.tokens = self.tokens.checked_add(&selected.revision.usage.into())?;
            }
            None => {
                self.requests_without_usage = self
                    .requests_without_usage
                    .checked_add(1)
                    .ok_or(TokenOverflow { category: "requests_without_usage" })?;
            }
        }
        Ok(())
    }
}

/// Why a set of totals is not complete.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PartialReason {
    /// A counted request has no usage revision.
    RequestWithoutUsage,
    /// Candidate-set members were left unresolved.
    UnresolvedUsage,
    /// Ambiguous requests lie partly inside the selection.
    PossibleUsage,
    /// Usage is known to exist but was not observed.
    UnobservedGap,
}

/// Whether totals cover everything they describe.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Completeness {
    /// Every counted request has usage, and nothing is unresolved, possible or unobserved.
    Complete,
    /// Something is missing, for these reasons.
    Partial(BTreeSet<PartialReason>),
}

impl Completeness {
    fn from_reasons(reasons: BTreeSet<PartialReason>) -> Self {
        if reasons.is_empty() { Self::Complete } else { Self::Partial(reasons) }
    }
}

/// Grand totals for a ledger.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LedgerTotals {
    /// Every counted request once.
    pub total: UsageTotals,
    /// Counted requests with a proven owner.
    pub owned: UsageTotals,
    /// Counted requests with candidate owners.
    pub ambiguous: UsageTotals,
    /// Counted requests with no owner evidence.
    pub unknown: UsageTotals,
    /// Unresolved candidate-set members; never added.
    pub unresolved: UsageTotals,
    /// Requests observed only as copies; never added.
    pub copy_only: UsageTotals,
    /// Whether the totals are complete.
    pub completeness: Completeness,
}

/// Computes grand totals.
pub fn ledger_totals(ledger: &Ledger) -> Result<LedgerTotals, TokenOverflow> {
    let mut totals = LedgerTotals {
        total: UsageTotals::default(),
        owned: UsageTotals::default(),
        ambiguous: UsageTotals::default(),
        unknown: UsageTotals::default(),
        unresolved: UsageTotals::default(),
        copy_only: UsageTotals::default(),
        completeness: Completeness::Complete,
    };
    for request in ledger.requests.values() {
        match request.counting {
            Counting::Counted => {
                totals.total.add(request)?;
                match request.ownership {
                    Ownership::Owned { .. } => totals.owned.add(request)?,
                    Ownership::Ambiguous { .. } => totals.ambiguous.add(request)?,
                    Ownership::Unknown => totals.unknown.add(request)?,
                }
            }
            Counting::Unresolved { .. } => totals.unresolved.add(request)?,
            Counting::CopyOnly => totals.copy_only.add(request)?,
        }
    }
    let mut reasons = BTreeSet::new();
    if totals.total.requests_without_usage > 0 {
        reasons.insert(PartialReason::RequestWithoutUsage);
    }
    if totals.unresolved.requests > 0 {
        reasons.insert(PartialReason::UnresolvedUsage);
    }
    if !ledger.gaps.is_empty() {
        reasons.insert(PartialReason::UnobservedGap);
    }
    totals.completeness = Completeness::from_reasons(reasons);
    Ok(totals)
}

/// Totals for a selection of threads.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SelectionTotals {
    /// Owned requests inside plus ambiguous requests wholly inside.
    pub counted: UsageTotals,
    /// Owned requests whose owner is selected.
    pub owned: UsageTotals,
    /// Ambiguous requests whose candidates are all selected.
    pub ambiguous: UsageTotals,
    /// Ambiguous requests with only some candidates selected; never added.
    pub possible: UsageTotals,
    /// Whether the selection's totals are complete.
    pub completeness: Completeness,
}

/// Computes totals for the requests a thread selection counts.
pub fn selection_totals(
    ledger: &Ledger,
    threads: &BTreeSet<AnalyticalId>,
) -> Result<SelectionTotals, TokenOverflow> {
    let mut totals = SelectionTotals {
        counted: UsageTotals::default(),
        owned: UsageTotals::default(),
        ambiguous: UsageTotals::default(),
        possible: UsageTotals::default(),
        completeness: Completeness::Complete,
    };
    let mut reasons = BTreeSet::new();
    for request in ledger.requests.values() {
        let inside = match &request.ownership {
            Ownership::Owned { thread } => threads.contains(thread).then_some(Placement::Owned),
            Ownership::Ambiguous { candidates } => {
                let selected = candidates.iter().filter(|c| threads.contains(*c)).count();
                if selected == 0 {
                    None
                } else if selected == candidates.len() {
                    Some(Placement::Ambiguous)
                } else {
                    Some(Placement::Possible)
                }
            }
            Ownership::Unknown => None,
        };
        let Some(placement) = inside else { continue };
        match request.counting {
            Counting::Counted => match placement {
                Placement::Owned => {
                    totals.owned.add(request)?;
                    totals.counted.add(request)?;
                }
                Placement::Ambiguous => {
                    totals.ambiguous.add(request)?;
                    totals.counted.add(request)?;
                }
                Placement::Possible => totals.possible.add(request)?,
            },
            Counting::Unresolved { .. } => {
                reasons.insert(PartialReason::UnresolvedUsage);
            }
            Counting::CopyOnly => {}
        }
    }
    if totals.counted.requests_without_usage > 0 {
        reasons.insert(PartialReason::RequestWithoutUsage);
    }
    if totals.possible.requests > 0 {
        reasons.insert(PartialReason::PossibleUsage);
    }
    let gap_applies = ledger
        .gaps
        .iter()
        .any(|gap| gap.thread.as_ref().is_none_or(|thread| threads.contains(thread)));
    if gap_applies {
        reasons.insert(PartialReason::UnobservedGap);
    }
    totals.completeness = Completeness::from_reasons(reasons);
    Ok(totals)
}

#[derive(Clone, Copy)]
enum Placement {
    Owned,
    Ambiguous,
    Possible,
}
