//! Sources and capture: dialect discovery, snapshot manifests, captured records and the
//! capture store (design §2).
//!
//! - [`manifest`] records each source's identity, extent, fingerprint, cutoff, counters
//!   and detected changes (§2.2).
//! - [`evidence`] defines the source ID, offset and length every record is cited by.

pub mod evidence;
pub mod manifest;
