//! Accounting: measures, ownership, time, usage windows and prices (design §4).
//!
//! - [`totals`] computes ownership-aware grand totals, thread selections, and the
//!   unresolved and possible measures that are never added (§4.2).

pub mod totals;
