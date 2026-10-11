//! Deep size: the heap a value owns, by the costing rule of [`super::model`].
//!
//! Admission charges a committed ledger, a session index and discovery metadata by their
//! deep size. Vectors, strings and boxes count their exact capacity; maps count the
//! model's upper bound, since the standard maps do not expose their node or bucket
//! layout. Interned names and overflow measures are process-wide and charged when they
//! are interned, not here.
//!
//! [`DeepSize::heap`] is what a value owns on the heap; [`DeepSize::deep_size`] adds the
//! value's own size, for a value that is itself on the heap, such as a vector element
//! counted by its vector, or one boxed. A value held on the stack, such as a function's
//! return value, is charged its heap only.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::SystemTime;

use super::model::{allocation, btree_map, sized_vec};
use crate::adapters::Ingested;
use crate::ledger::coverage::{CoverageGap, ReconcileCoverage, UnobservedReason};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{
    Basis, CompactTimestamp, Confidence, Counting, ModelBasis, ModelName, ModelUsage, Ownership,
    ProviderLimitObservation, RecordRefs, Relationship, RelationshipKind, Request, RevisionStatus,
    SelectedUsage, SourceArtifact, SourceCapability, Thread, ToolAction, UsageRevision,
};
use crate::ledger::identity::{
    AnalyticalId, IdPrefix, IdentityKey, IdentityVersion, KeyComponent, StoredIdentity,
};
use crate::ledger::names::Name;
use crate::ledger::reconcile::{
    Ledger, LineageLink, NativeSequence, ObservationRole, OwnerEvidence, ReconcileInput,
    RequestObservation,
};
use crate::ledger::scope::{DerivedKey, IdentityBasis};
use crate::ledger::tokens::Measures;
use crate::selection::{Agent, IndexedSession};
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::{
    CoverageFailure, Cutoff, FileIdentity, Fingerprint, ManifestEntry, RecordCounters,
    Representation, SkippedLink, SkippedLinkReason, SnapshotManifest, SourceChange,
};
use crate::sources::reader::LogicalSource;
use crate::sources::roots::{DiscoveredSource, Discovery, UnreadableEntry};

/// A value whose owned heap the model can bound.
pub trait DeepSize {
    /// The heap this value owns beyond its own `size_of`, by the costing rule.
    fn heap(&self) -> u64;

    /// `size_of` plus [`DeepSize::heap`]: what a value costs where it is itself on the
    /// heap.
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

// Plain values, which own nothing on the heap.
owns_no_heap!(
    u8,
    u16,
    u32,
    u64,
    usize,
    bool,
    &'static str,
    SystemTime,
    io::ErrorKind,
    jiff::Timestamp,
    Agent,
    AnalyticalId,
    CompactTimestamp,
    Confidence,
    Cutoff,
    DerivedKey,
    DiagnosticCode,
    EvidenceRef,
    Fingerprint,
    IdPrefix,
    IdentityBasis,
    IdentityVersion,
    Measures,
    ModelBasis,
    ModelName,
    Name,
    NativeSequence,
    ObservationRole,
    ReconcileCoverage,
    RecordCounters,
    Representation,
    RevisionStatus,
    SkippedLinkReason,
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

/// Adds up the heap of every listed field.
fn fields(heaps: impl IntoIterator<Item = u64>) -> u64 {
    heaps.into_iter().fold(0, u64::saturating_add)
}

// Every struct below is destructured without `..`, and every field's heap is added, so a
// new field fails to compile until it is sized here.

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
        let Self { prefix, version, kind, components } = self;
        fields([prefix.heap(), version.heap(), kind.heap(), components.heap()])
    }
}

impl DeepSize for StoredIdentity {
    fn heap(&self) -> u64 {
        let Self { id, key } = self;
        fields([id.heap(), key.heap()])
    }
}

impl DeepSize for Thread {
    fn heap(&self) -> u64 {
        let Self {
            identity,
            basis,
            aliases,
            native_key,
            source,
            initiator,
            purpose,
            execution_environment,
            project,
            account,
            evidence,
        } = self;
        fields([
            identity.heap(),
            basis.heap(),
            aliases.heap(),
            native_key.heap(),
            source.heap(),
            initiator.heap(),
            purpose.heap(),
            execution_environment.heap(),
            project.heap(),
            account.heap(),
            evidence.heap(),
        ])
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
        let Self { kind, from, to, confidence, evidence } = self;
        fields([kind.heap(), from.heap(), to.heap(), confidence.heap(), evidence.heap()])
    }
}

impl DeepSize for Ownership {
    fn heap(&self) -> u64 {
        match self {
            Self::Owned { thread } => thread.heap(),
            Self::Ambiguous { candidates } => candidates.heap(),
            Self::Unknown => 0,
        }
    }
}

impl DeepSize for Counting {
    fn heap(&self) -> u64 {
        match self {
            Self::Unresolved { counted } => counted.heap(),
            Self::Counted | Self::CopyOnly => 0,
        }
    }
}

impl DeepSize for OwnerEvidence {
    fn heap(&self) -> u64 {
        match self {
            Self::Proven(thread) => thread.heap(),
            Self::None => 0,
        }
    }
}

impl DeepSize for ModelUsage {
    fn heap(&self) -> u64 {
        let Self { model, usage, source } = self;
        fields([model.heap(), usage.heap(), source.heap()])
    }
}

impl DeepSize for UsageRevision {
    fn heap(&self) -> u64 {
        let Self { usage, model_usage } = self;
        fields([usage.heap(), model_usage.heap()])
    }
}

impl DeepSize for SelectedUsage {
    fn heap(&self) -> u64 {
        let Self { revision, evidence, status } = self;
        fields([revision.heap(), evidence.heap(), status.heap()])
    }
}

impl DeepSize for RecordRefs {
    fn heap(&self) -> u64 {
        match self {
            Self::Many(records) => records.heap(),
            Self::One(record) => record.heap(),
            Self::Empty => 0,
        }
    }
}

impl DeepSize for Request {
    fn heap(&self) -> u64 {
        let Self {
            id,
            basis,
            aliases,
            ownership,
            first_seen,
            last_seen,
            model,
            effort,
            usage,
            records,
            originals,
            counting,
        } = self;
        fields([
            id.heap(),
            basis.heap(),
            aliases.heap(),
            ownership.heap(),
            first_seen.heap(),
            last_seen.heap(),
            model.heap(),
            effort.heap(),
            usage.heap(),
            records.heap(),
            originals.heap(),
            counting.heap(),
        ])
    }
}

impl DeepSize for ToolAction {
    fn heap(&self) -> u64 {
        let Self { identity, basis, call_id, tool_name, request, evidence } = self;
        fields([
            identity.heap(),
            basis.heap(),
            call_id.heap(),
            tool_name.heap(),
            request.heap(),
            evidence.heap(),
        ])
    }
}

impl DeepSize for ProviderLimitObservation {
    fn heap(&self) -> u64 {
        let Self { limit_name, window, observed_at, owner_thread, owner_request, native, evidence } =
            self;
        fields([
            limit_name.heap(),
            window.heap(),
            observed_at.heap(),
            owner_thread.heap(),
            owner_request.heap(),
            native.heap(),
            evidence.heap(),
        ])
    }
}

impl DeepSize for Diagnostic {
    fn heap(&self) -> u64 {
        let Self { code, subject, evidence, occurrences, detail } = self;
        fields([code.heap(), subject.heap(), evidence.heap(), occurrences.heap(), detail.heap()])
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
        let Self { reason, thread, evidence } = self;
        fields([reason.heap(), thread.heap(), evidence.heap()])
    }
}

impl DeepSize for Ledger {
    fn heap(&self) -> u64 {
        let Self {
            threads,
            relationships,
            requests,
            tool_actions,
            limit_observations,
            candidate_sets,
            revision_rule,
            diagnostics,
            gaps,
            coverage,
            source_table,
        } = self;
        fields([
            threads.heap(),
            relationships.heap(),
            requests.heap(),
            tool_actions.heap(),
            limit_observations.heap(),
            candidate_sets.heap(),
            revision_rule.heap(),
            diagnostics.heap(),
            gaps.heap(),
            coverage.heap(),
            source_table.heap(),
        ])
    }
}

impl DeepSize for RequestObservation {
    fn heap(&self) -> u64 {
        let Self {
            evidence,
            keys,
            role,
            owner,
            usage,
            model_usage,
            sequence,
            invariants,
            model,
            effort,
            timestamp,
        } = self;
        fields([
            evidence.heap(),
            keys.heap(),
            role.heap(),
            owner.heap(),
            usage.heap(),
            model_usage.heap(),
            sequence.heap(),
            invariants.heap(),
            model.heap(),
            effort.heap(),
            timestamp.heap(),
        ])
    }
}

impl DeepSize for LineageLink {
    fn heap(&self) -> u64 {
        let Self { a, b, evidence } = self;
        fields([a.heap(), b.heap(), evidence.heap()])
    }
}

impl DeepSize for ReconcileInput {
    fn heap(&self) -> u64 {
        let Self {
            threads,
            relationships,
            requests,
            tool_actions,
            limit_observations,
            links,
            gaps,
            diagnostics,
            source_table,
        } = self;
        fields([
            threads.heap(),
            relationships.heap(),
            requests.heap(),
            tool_actions.heap(),
            limit_observations.heap(),
            links.heap(),
            gaps.heap(),
            diagnostics.heap(),
            source_table.heap(),
        ])
    }
}

impl DeepSize for FileIdentity {
    fn heap(&self) -> u64 {
        let Self { path, device, inode } = self;
        fields([path.heap(), device.heap(), inode.heap()])
    }
}

impl DeepSize for CoverageFailure {
    fn heap(&self) -> u64 {
        match self {
            Self::CorruptCompressedData { decoded_offset, message } => {
                fields([decoded_offset.heap(), message.heap()])
            }
            Self::TwinFingerprintMismatch { path, locator }
            | Self::UnreadableTwin { path, locator } => fields([path.heap(), locator.heap()]),
            Self::Oversized { .. }
            | Self::IncompleteCompressedFrame { .. }
            | Self::ReadError { .. } => 0,
        }
    }
}

impl DeepSize for SourceChange {
    fn heap(&self) -> u64 {
        match self {
            Self::ReadFromOtherRepresentation { primary, representation } => {
                fields([primary.heap(), representation.heap()])
            }
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
        let Self {
            source,
            environment,
            dialect,
            locator,
            file,
            representation,
            twins,
            file_len,
            modified,
            fingerprint,
            cutoff,
            counters,
            first_malformed,
            failures,
            changes,
        } = self;
        fields([
            source.heap(),
            environment.heap(),
            dialect.heap(),
            locator.heap(),
            file.heap(),
            representation.heap(),
            twins.heap(),
            file_len.heap(),
            modified.heap(),
            fingerprint.heap(),
            cutoff.heap(),
            counters.heap(),
            first_malformed.heap(),
            failures.heap(),
            changes.heap(),
        ])
    }
}

impl DeepSize for SkippedLink {
    fn heap(&self) -> u64 {
        let Self { path, reason } = self;
        fields([path.heap(), reason.heap()])
    }
}

impl DeepSize for SnapshotManifest {
    fn heap(&self) -> u64 {
        let Self { entries, skipped_links } = self;
        fields([entries.heap(), skipped_links.heap()])
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
        let Self { dialect_version, capability, snapshot } = self;
        fields([dialect_version.heap(), capability.heap(), snapshot.heap()])
    }
}

impl DeepSize for Ingested {
    fn heap(&self) -> u64 {
        let Self { manifest, sources, threads, relationships, ledger, limit_observations } = self;
        fields([
            manifest.heap(),
            sources.heap(),
            threads.heap(),
            relationships.heap(),
            ledger.heap(),
            limit_observations.heap(),
        ])
    }
}

impl DeepSize for IndexedSession {
    fn heap(&self) -> u64 {
        let Self { thread, agent, source_paths } = self;
        fields([thread.heap(), agent.heap(), source_paths.heap()])
    }
}

impl DeepSize for LogicalSource {
    fn heap(&self) -> u64 {
        let Self { plain, zstd, gzip } = self;
        fields([plain.heap(), zstd.heap(), gzip.heap()])
    }
}

impl DeepSize for DiscoveredSource {
    fn heap(&self) -> u64 {
        let Self { root, locator, files } = self;
        fields([root.heap(), locator.heap(), files.heap()])
    }
}

impl DeepSize for UnreadableEntry {
    fn heap(&self) -> u64 {
        let Self { path, kind } = self;
        fields([path.heap(), kind.heap()])
    }
}

impl DeepSize for Discovery {
    fn heap(&self) -> u64 {
        let Self { sources, skipped_links, missing_roots, unreadable, duplicate_paths } = self;
        fields([
            sources.heap(),
            skipped_links.heap(),
            missing_roots.heap(),
            unreadable.heap(),
            duplicate_paths.heap(),
        ])
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
