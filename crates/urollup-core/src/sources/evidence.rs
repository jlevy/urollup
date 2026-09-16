//! Evidence references: where a record sits inside a snapshot (design §2.2).

use crate::ledger::identity::AnalyticalId;

/// A record's location: its source's `src-` ID, byte offset and length.
///
/// Offsets and lengths count decoded bytes, so a `.jsonl` source and its `.jsonl.zst` twin
/// give one record the same reference. The length excludes the line terminator.
/// The derived order (source, offset, length) is the canonical record order that
/// reconciliation uses in place of traversal order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EvidenceRef {
    /// The source artifact's `src-` ID.
    pub source: AnalyticalId,
    /// Byte offset of the record's first byte in the decoded source.
    pub offset: u64,
    /// Record length in bytes, excluding the line terminator.
    pub length: u64,
}
