//! Sources and capture: dialect discovery, snapshot manifests, captured records and the
//! capture store (design §2).
//!
//! - [`manifest`] records each source's identity, extent, fingerprint, cutoff, counters
//!   and detected changes (§2.2).
//! - [`evidence`] defines the source ID, offset and length every record is cited by.
//! - [`reader`] streams complete records within a snapshot's extent and detects the
//!   changes, tails and failures §2.2 requires; [`decode`] holds the lenient decoding
//!   helpers, [`prefilter`] the line hint and [`parallel`] the bounded,
//!   order-preserving multi-file read; [`roots`] walks declared roots, following symlinks
//!   only inside them (§2.1).

pub mod decode;
pub mod evidence;
pub mod manifest;
pub mod parallel;
pub mod prefilter;
pub mod reader;
pub mod roots;
