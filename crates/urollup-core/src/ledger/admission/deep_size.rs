//! Deep size: the heap a value owns, by the costing rule of [`super::model`].
//!
//! Admission charges a committed ledger, a session index and discovery metadata by their
//! deep size. Vectors, strings and boxes count their exact capacity; maps count the
//! model's upper bound, since the standard maps do not expose their node or bucket
//! layout. Interned names and overflow measures are process-wide and charged when they
//! are interned, not here.

use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;
use std::path::PathBuf;

use super::model::{allocation, btree_map, sized_vec};
use crate::adapters::Ingested;
use crate::ledger::coverage::{CoverageGap, UnobservedReason};
use crate::ledger::diagnostics::Diagnostic;
use crate::ledger::entities::{
    Basis, Counting, ModelUsage, Ownership, ProviderLimitObservation, RecordRefs, Relationship,
    RelationshipKind, Request, SelectedUsage, SourceArtifact, SourceCapability, Thread, ToolAction,
};
use crate::ledger::identity::{AnalyticalId, IdentityKey, KeyComponent, StoredIdentity};
use crate::ledger::names::Name;
use crate::ledger::reconcile::{Ledger, LineageLink, ReconcileInput, RequestObservation};
use crate::ledger::scope::DerivedKey;
use crate::selection::IndexedSession;
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::{
    CoverageFailure, FileIdentity, ManifestEntry, SkippedLink, SnapshotManifest, SourceChange,
};
use crate::sources::reader::LogicalSource;
use crate::sources::roots::{DiscoveredSource, Discovery, UnreadableEntry};

/// A value whose owned heap the model can bound.
pub trait DeepSize {
    /// The heap this value owns beyond its own `size_of`, by the costing rule.
    fn heap(&self) -> u64;

    /// `size_of` plus [`DeepSize::heap`]: what a value costs where it is not part of a
    /// larger allocation.
    fn deep_size(&self) -> u64
    where
        Self: Sized,
    {
        (size_of::<Self>() as u64).saturating_add(self.heap())
    }
}

macro_rules! owns_no_heap {
    ($($type:ty),* $(,)?) => {
        $(impl DeepSize for $type {
            fn heap(&self) -> u64 {
                0
            }
        })*
    };
}

owns_no_heap!(
    u8,
    u16,
    u32,
    u64,
    usize,
    bool,
    &'static str,
    AnalyticalId,
    DerivedKey,
    EvidenceRef,
    Name,
    ModelUsage,
    Counting,
    ProviderLimitObservation,
    jiff::Timestamp,
);

fn sum<'a, T: DeepSize + 'a>(items: impl IntoIterator<Item = &'a T>) -> u64 {
    items.into_iter().fold(0, |total, item| total.saturating_add(item.heap()))
}

fn size<T>() -> u64 {
    size_of::<T>() as u64
}

impl DeepSize for String {
    fn heap(&self) -> u64 {
        allocation(self.capacity() as u64)
    }
}

impl DeepSize for PathBuf {
    fn heap(&self) -> u64 {
        allocation(self.capacity() as u64)
    }
}

impl DeepSize for Box<str> {
    fn heap(&self) -> u64 {
        allocation(self.len() as u64)
    }
}

impl<T: DeepSize> DeepSize for Vec<T> {
    fn heap(&self) -> u64 {
        sized_vec(self.capacity() as u64, size::<T>()).saturating_add(sum(self))
    }
}

impl<T: DeepSize> DeepSize for Box<[T]> {
    fn heap(&self) -> u64 {
        sized_vec(self.len() as u64, size::<T>()).saturating_add(sum(self.iter()))
    }
}

impl<T: DeepSize> DeepSize for Option<T> {
    fn heap(&self) -> u64 {
        self.as_ref().map_or(0, DeepSize::heap)
    }
}

impl<A: DeepSize, B: DeepSize> DeepSize for (A, B) {
    fn heap(&self) -> u64 {
        self.0.heap().saturating_add(self.1.heap())
    }
}

impl<K: DeepSize, V: DeepSize> DeepSize for BTreeMap<K, V> {
    fn heap(&self) -> u64 {
        self.iter().fold(btree_map(self.len() as u64, size::<K>(), size::<V>()), |total, (k, v)| {
            total.saturating_add(k.heap()).saturating_add(v.heap())
        })
    }
}

impl<T: DeepSize> DeepSize for BTreeSet<T> {
    fn heap(&self) -> u64 {
        btree_map(self.len() as u64, size::<T>(), 0).saturating_add(sum(self))
    }
}

impl<T: DeepSize> DeepSize for Basis<T> {
    fn heap(&self) -> u64 {
        self.value().map_or(0, DeepSize::heap)
    }
}

impl DeepSize for KeyComponent {
    fn heap(&self) -> u64 {
        match self {
            Self::Text(text) => text.heap(),
            Self::Null | Self::Integer(_) | Self::Redacted => 0,
        }
    }
}

impl DeepSize for IdentityKey {
    fn heap(&self) -> u64 {
        self.kind.heap().saturating_add(self.components.heap())
    }
}

impl DeepSize for StoredIdentity {
    fn heap(&self) -> u64 {
        self.key.heap()
    }
}

impl DeepSize for Thread {
    fn heap(&self) -> u64 {
        [
            self.identity.heap(),
            self.aliases.heap(),
            self.native_key.heap(),
            self.source.heap(),
            self.initiator.heap(),
            self.purpose.heap(),
            self.execution_environment.heap(),
            self.project.heap(),
            self.account.heap(),
            self.evidence.heap(),
        ]
        .into_iter()
        .fold(0, u64::saturating_add)
    }
}

impl DeepSize for RelationshipKind {
    fn heap(&self) -> u64 {
        match self {
            Self::Other(token) => token.heap(),
            Self::Spawn | Self::Fork | Self::Resume | Self::Review | Self::InlineSidechain => 0,
        }
    }
}

impl DeepSize for Relationship {
    fn heap(&self) -> u64 {
        self.kind.heap().saturating_add(self.evidence.heap())
    }
}

impl DeepSize for Ownership {
    fn heap(&self) -> u64 {
        match self {
            Self::Ambiguous { candidates } => candidates.heap(),
            Self::Owned { .. } | Self::Unknown => 0,
        }
    }
}

impl DeepSize for SelectedUsage {
    fn heap(&self) -> u64 {
        self.revision.model_usage.heap()
    }
}

impl DeepSize for RecordRefs {
    fn heap(&self) -> u64 {
        match self {
            Self::Many(records) => records.heap(),
            Self::Empty | Self::One(_) => 0,
        }
    }
}

impl DeepSize for Request {
    fn heap(&self) -> u64 {
        [self.aliases.heap(), self.ownership.heap(), self.usage.heap(), self.records.heap()]
            .into_iter()
            .fold(0, u64::saturating_add)
    }
}

impl DeepSize for ToolAction {
    fn heap(&self) -> u64 {
        [self.identity.heap(), self.call_id.heap(), self.tool_name.heap(), self.evidence.heap()]
            .into_iter()
            .fold(0, u64::saturating_add)
    }
}

impl DeepSize for Diagnostic {
    fn heap(&self) -> u64 {
        self.evidence.heap().saturating_add(self.detail.heap())
    }
}

impl DeepSize for UnobservedReason {
    fn heap(&self) -> u64 {
        match self {
            Self::Other(token) => token.heap(),
            Self::EphemeralThread
            | Self::ParallelGuardianReview
            | Self::LegacyRemoteCompaction
            | Self::UnreadableSource
            | Self::UnverifiedHistoryBoundary => 0,
        }
    }
}

impl DeepSize for CoverageGap {
    fn heap(&self) -> u64 {
        self.reason.heap().saturating_add(self.evidence.heap())
    }
}

impl DeepSize for Ledger {
    fn heap(&self) -> u64 {
        [
            self.threads.heap(),
            self.relationships.heap(),
            self.requests.heap(),
            self.tool_actions.heap(),
            self.limit_observations.heap(),
            self.candidate_sets.heap(),
            self.diagnostics.heap(),
            self.gaps.heap(),
            self.source_table.heap(),
        ]
        .into_iter()
        .fold(0, u64::saturating_add)
    }
}

impl DeepSize for RequestObservation {
    fn heap(&self) -> u64 {
        [self.keys.heap(), self.model_usage.heap(), self.invariants.heap()]
            .into_iter()
            .fold(0, u64::saturating_add)
    }
}

impl DeepSize for LineageLink {
    fn heap(&self) -> u64 {
        self.evidence.heap()
    }
}

impl DeepSize for ReconcileInput {
    fn heap(&self) -> u64 {
        [
            self.threads.heap(),
            self.relationships.heap(),
            self.requests.heap(),
            self.tool_actions.heap(),
            self.limit_observations.heap(),
            self.links.heap(),
            self.gaps.heap(),
            self.diagnostics.heap(),
            self.source_table.heap(),
        ]
        .into_iter()
        .fold(0, u64::saturating_add)
    }
}

impl DeepSize for FileIdentity {
    fn heap(&self) -> u64 {
        self.path.heap()
    }
}

impl DeepSize for CoverageFailure {
    fn heap(&self) -> u64 {
        match self {
            Self::CorruptCompressedData { message, .. } => message.heap(),
            Self::TwinFingerprintMismatch { path, locator }
            | Self::UnreadableTwin { path, locator } => path.heap().saturating_add(locator.heap()),
            Self::Oversized { .. }
            | Self::IncompleteCompressedFrame { .. }
            | Self::ReadError { .. } => 0,
        }
    }
}

impl DeepSize for SourceChange {
    fn heap(&self) -> u64 {
        match self {
            Self::ReadFromOtherRepresentation { primary, .. } => primary.heap(),
            Self::BrieflyAbsent { .. }
            | Self::Vanished
            | Self::RemovedAfterScan
            | Self::Replaced
            | Self::Truncated { .. }
            | Self::ModifiedInPlace
            | Self::FirstRecordChanged
            | Self::GrewBeyondCutoff { .. } => 0,
        }
    }
}

impl DeepSize for ManifestEntry {
    fn heap(&self) -> u64 {
        [
            self.source.heap(),
            self.environment.heap(),
            self.dialect.heap(),
            self.locator.heap(),
            self.file.heap(),
            self.twins.heap(),
            self.failures.heap(),
            self.changes.heap(),
        ]
        .into_iter()
        .fold(0, u64::saturating_add)
    }
}

impl DeepSize for SkippedLink {
    fn heap(&self) -> u64 {
        self.path.heap()
    }
}

impl DeepSize for SnapshotManifest {
    fn heap(&self) -> u64 {
        self.entries.heap().saturating_add(self.skipped_links.heap())
    }
}

impl DeepSize for SourceCapability {
    fn heap(&self) -> u64 {
        match self {
            Self::Unsupported(reason) => reason.heap(),
            Self::Supported => 0,
        }
    }
}

impl DeepSize for SourceArtifact {
    fn heap(&self) -> u64 {
        [self.dialect_version.heap(), self.capability.heap(), self.snapshot.heap()]
            .into_iter()
            .fold(0, u64::saturating_add)
    }
}

impl DeepSize for Ingested {
    fn heap(&self) -> u64 {
        [
            self.manifest.heap(),
            self.sources.heap(),
            self.threads.heap(),
            self.relationships.heap(),
            self.ledger.heap(),
            self.limit_observations.heap(),
        ]
        .into_iter()
        .fold(0, u64::saturating_add)
    }
}

impl DeepSize for IndexedSession {
    fn heap(&self) -> u64 {
        self.thread.heap().saturating_add(self.source_paths.heap())
    }
}

impl DeepSize for LogicalSource {
    fn heap(&self) -> u64 {
        [self.plain.heap(), self.zstd.heap(), self.gzip.heap()]
            .into_iter()
            .fold(0, u64::saturating_add)
    }
}

impl DeepSize for DiscoveredSource {
    fn heap(&self) -> u64 {
        [self.root.heap(), self.locator.heap(), self.files.heap()]
            .into_iter()
            .fold(0, u64::saturating_add)
    }
}

impl DeepSize for UnreadableEntry {
    fn heap(&self) -> u64 {
        self.path.heap()
    }
}

impl DeepSize for Discovery {
    fn heap(&self) -> u64 {
        [
            self.sources.heap(),
            self.skipped_links.heap(),
            self.missing_roots.heap(),
            self.unreadable.heap(),
            self.duplicate_paths.heap(),
        ]
        .into_iter()
        .fold(0, u64::saturating_add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_and_vectors_count_their_capacity_by_the_costing_rule() {
        let mut text = String::with_capacity(20);
        text.push('a');
        assert_eq!(text.heap(), 48);
        let names: Vec<String> = vec![String::from("abc"), String::new()];
        assert_eq!(names.heap(), sized_vec(names.capacity() as u64, 24) + 32);
        assert_eq!(Vec::<u64>::new().heap(), 0);
        assert_eq!(Some(String::from("abc")).heap(), 32);
        assert_eq!(text.deep_size(), 24 + 48);
    }

    #[test]
    fn maps_count_their_node_bound_and_their_entries() {
        let map: BTreeMap<String, String> =
            [(String::from("a"), String::from("b"))].into_iter().collect();
        assert_eq!(map.heap(), btree_map(1, 24, 24) + 64);
        assert_eq!(BTreeSet::<u32>::new().heap(), 0);
    }
}
