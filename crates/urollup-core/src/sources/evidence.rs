//! Evidence references: where a record sits inside a snapshot (design §2.2).

use crate::ledger::identity::AnalyticalId;

/// A record's location: a source-table index, byte offset and length.
///
/// Offsets and lengths count decoded bytes, so a `.jsonl` source and its `.jsonl.zst` twin
/// give one record the same reference. The length excludes the line terminator.
/// The derived order (source, offset, length) is the canonical record order that
/// reconciliation uses in place of traversal order.
///
/// Sixteen bytes: `source` and `length` are `u32`, `offset` is `u64`. Length fits because
/// the reader refuses records larger than 64 MiB. `source` is an index into the
/// [`SourceTable`] that held the `src-` IDs when the reference was built. Indices are
/// assigned in `AnalyticalId` order so this order matches the former embedded-ID order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EvidenceRef {
    /// Index into the source table that names this record's `src-` ID.
    pub source: u32,
    /// Record length in bytes, excluding the line terminator.
    pub length: u32,
    /// Byte offset of the record's first byte in the decoded source.
    pub offset: u64,
}

impl PartialOrd for EvidenceRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EvidenceRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Canonical record order is source, then offset, then length, not struct layout.
        self.source
            .cmp(&other.source)
            .then(self.offset.cmp(&other.offset))
            .then(self.length.cmp(&other.length))
    }
}

const _: () = assert!(std::mem::size_of::<EvidenceRef>() == 16);

impl EvidenceRef {
    /// A reference to `length` decoded bytes at `offset` in `source`.
    ///
    /// `length` must fit in `u32`; the reader never buffers a longer record.
    pub fn new(source: u32, offset: u64, length: u64) -> Self {
        Self { source, offset, length: u32::try_from(length).expect("record length fits u32") }
    }

    /// The same location in `source`.
    #[must_use]
    pub fn with_source(self, source: u32) -> Self {
        Self { source, ..self }
    }
}

/// The `src-` IDs an [`EvidenceRef`] names, in `AnalyticalId` order.
///
/// Adapters collect every source ID in one ingest, build this table once, then stamp
/// references with [`SourceTable::index_of`]. Incremental insert would shift earlier
/// indices, so the table is frozen after [`SourceTable::from_ids`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceTable {
    ids: Vec<AnalyticalId>,
}

impl SourceTable {
    /// IDs in the given order, so index `n` is `ids[n]`.
    ///
    /// Production ingest uses [`SourceTable::from_ids`]. Tests that cite sources by small
    /// integers use this so `EvidenceRef.source` is that integer.
    pub fn from_ordered(ids: Vec<AnalyticalId>) -> Self {
        Self { ids }
    }

    /// Unique IDs in analytical-ID order, so evidence order is unchanged.
    pub fn from_ids(ids: impl IntoIterator<Item = AnalyticalId>) -> Self {
        let mut ids: Vec<AnalyticalId> = ids.into_iter().collect();
        ids.sort();
        ids.dedup();
        Self { ids }
    }

    /// The index of `id`, when it was in the set passed to [`SourceTable::from_ids`].
    pub fn index_of(&self, id: &AnalyticalId) -> Option<u32> {
        self.ids.binary_search(id).ok().and_then(|index| u32::try_from(index).ok())
    }

    /// The `src-` ID at `index`.
    pub fn get(&self, index: u32) -> Option<&AnalyticalId> {
        self.ids.get(usize::try_from(index).ok()?)
    }

    /// Every ID, in index order.
    pub fn ids(&self) -> &[AnalyticalId] {
        &self.ids
    }
}

#[cfg(test)]
mod tests {
    use super::{EvidenceRef, SourceTable};
    use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent};

    fn source(n: u8) -> AnalyticalId {
        IdentityKey::new(IdPrefix::Source, "test-source", vec![KeyComponent::Integer(i64::from(n))])
            .derive_id()
            .unwrap()
    }

    #[test]
    fn a_reference_is_sixteen_bytes() {
        assert_eq!(std::mem::size_of::<EvidenceRef>(), 16);
    }

    #[test]
    fn order_is_source_then_offset_then_length() {
        let earlier = EvidenceRef::new(0, 10, 20);
        let later = EvidenceRef::new(0, 11, 5);
        assert!(earlier < later);
    }

    #[test]
    fn indices_follow_analytical_id_order() {
        let first = source(1);
        let second = source(2);
        let table = SourceTable::from_ids([second.clone(), first.clone()]);
        let left = table.index_of(&first).unwrap();
        let right = table.index_of(&second).unwrap();
        assert_eq!(left < right, first < second);
        assert_eq!(table.get(left), Some(&first));
        assert_eq!(table.get(right), Some(&second));
    }
}
