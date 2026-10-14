//! Persistent agent-log dialect adapters.
//!
//! Adapters turn source snapshots into the normalized entities and reconciled request
//! ledger shared by every report. They preserve source evidence and keep discovery and
//! decoding separate from selection and accounting.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::ledger::admission::{CapacityError, Component, Hold, MemoryAdmission, Phase, model};
use crate::ledger::coverage::{CoverageGap, UnobservedReason};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{ProviderLimitObservation, Relationship, SourceArtifact, Thread};
use crate::ledger::identity::AnalyticalId;
use crate::ledger::reconcile::{Ledger, ReconcileError};
use crate::selection::Agent;
use crate::sources::evidence::EvidenceRef;
use crate::sources::manifest::{ManifestEntry, Representation, SnapshotManifest};
use crate::sources::reader::SourceReadError;
use crate::sources::roots::Discovery;

pub mod claude_project;
pub mod codex_rollout;
pub mod discovery;

/// The normalized result of ingesting one or more roots of one dialect.
///
/// Threads, relationships and limit observations are moved out of the reconciled ledger
/// into their own fields rather than copied, so the ledger's copies of them are empty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ingested {
    /// Every source snapshot read.
    pub manifest: SnapshotManifest,
    /// Source entities, in manifest order.
    pub sources: Vec<SourceArtifact>,
    /// Threads by analytical ID.
    pub threads: BTreeMap<AnalyticalId, Thread>,
    /// Native relationships between threads.
    pub relationships: Vec<Relationship>,
    /// Reconciled logical requests.
    pub ledger: Ledger,
    /// Provider usage-limit observations.
    pub limit_observations: Vec<ProviderLimitObservation>,
}

impl Ingested {
    /// Drops source, thread and relationship tables after
    /// [`crate::selection::SessionIndex`] has copied what selection needs.
    ///
    /// Query reads the ledger and limit observations. Discovery tables would otherwise
    /// stay resident while the next dialect ingests, which is the whole-history peak.
    pub fn release_discovery(&mut self) {
        self.manifest.entries.clear();
        self.manifest.entries.shrink_to_fit();
        self.manifest.skipped_links.clear();
        self.manifest.skipped_links.shrink_to_fit();
        self.sources.clear();
        self.sources.shrink_to_fit();
        self.threads = BTreeMap::new();
        self.relationships.clear();
        self.relationships.shrink_to_fit();
    }
}

/// A single `source-incomplete` diagnostic covering every snapshot that lost data, and an
/// unreadable-source coverage gap for each, so a damaged, truncated, replaced or vanished
/// source makes totals partial instead of silently smaller.
///
/// The diagnostic's occurrences count the losing sources, and its detail names each kind
/// of loss with the number of sources that had it, such as "3 Codex rollouts could not be
/// read completely: corrupt-compressed-data (2), incomplete-compressed-frame (1)". It is
/// one diagnostic because [`crate::ledger::diagnostics::compact`] keeps a single detail
/// per code, and sources without a `src-` ID cite nothing that tells their diagnostics
/// apart. `evidence` cites a source by its `src-` ID; a source with no complete record has
/// none. `kind` names the dialect's sources in the detail, which holds no path.
pub(crate) fn snapshot_losses(
    manifest: &SnapshotManifest,
    kind: &str,
    evidence: impl Fn(&ManifestEntry) -> Option<EvidenceRef>,
) -> (Option<Diagnostic>, Vec<CoverageGap>) {
    let mut sources: u64 = 0;
    let mut by_loss: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut cited = Vec::new();
    let mut gaps = Vec::new();
    for entry in &manifest.entries {
        let losses = entry.losses();
        if losses.is_empty() {
            continue;
        }
        sources = sources.saturating_add(1);
        for loss in losses {
            let count = by_loss.entry(loss).or_default();
            *count = count.saturating_add(1);
        }
        let evidence: Vec<EvidenceRef> = evidence(entry).into_iter().collect();
        cited.extend(evidence.iter().copied());
        gaps.push(CoverageGap {
            reason: UnobservedReason::UnreadableSource,
            thread: None,
            evidence,
        });
    }
    let detail = match sources {
        0 => return (None, gaps),
        1 => format!(
            "a {kind} could not be read completely: {}",
            by_loss.into_keys().collect::<Vec<_>>().join(", ")
        ),
        _ => format!(
            "{sources} {kind}s could not be read completely: {}",
            by_loss
                .iter()
                .map(|(loss, count)| format!("{loss} ({count})"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let diagnostic = Diagnostic::new(DiagnosticCode::SourceIncomplete, None, cited, detail)
        .with_occurrences(sources);
    (Some(diagnostic), gaps)
}

/// A persistent-log adapter could not produce a normalized result.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// A declared root does not exist.
    #[error("source root does not exist: {0}")]
    MissingRoot(PathBuf),
    /// A path below a source root could not be inspected.
    #[error("cannot inspect source path {path}: {kind:?}")]
    UnreadablePath {
        /// The path discovery could not inspect.
        path: PathBuf,
        /// The operating-system error category.
        kind: std::io::ErrorKind,
    },
    /// A source could not be snapshotted.
    #[error("failed to read source {path}: {source}")]
    Read {
        /// The source path.
        path: PathBuf,
        /// Why it could not be read.
        #[source]
        source: SourceReadError,
    },
    /// A Claude subagent metadata sidecar could not be read.
    #[error("cannot read Claude subagent metadata {path}: {source}")]
    MetadataRead {
        /// The sidecar path.
        path: PathBuf,
        /// Why it could not be read.
        #[source]
        source: std::io::Error,
    },
    /// A Claude subagent metadata sidecar exceeds the adapter's hard size limit.
    #[error("Claude subagent metadata {path} exceeds the {maximum}-byte size limit")]
    MetadataTooLarge {
        /// The sidecar path.
        path: PathBuf,
        /// The maximum bytes accepted for one sidecar.
        maximum: u64,
    },
    /// A Claude subagent metadata sidecar is not valid JSON.
    #[error("cannot parse Claude subagent metadata {path}: {source}")]
    MetadataParse {
        /// The sidecar path.
        path: PathBuf,
        /// Why it could not be decoded.
        #[source]
        source: serde_json::Error,
    },
    /// An analytical identity could not be derived.
    #[error(transparent)]
    Identity(#[from] crate::ledger::identity::IdentityError),
    /// A scoped identity key could not be built.
    #[error(transparent)]
    Key(#[from] crate::ledger::scope::KeyScopeError),
    /// Request observations could not be reconciled.
    #[error(transparent)]
    Reconcile(#[from] ReconcileError),
    /// Process-wide memory admission refused the invocation. A row-ceiling refusal
    /// converts to [`ReconcileError::CapacityExceeded`] instead, which keeps its message.
    #[error(transparent)]
    Capacity(CapacityError),
    /// Normalized token arithmetic overflowed.
    #[error(transparent)]
    Tokens(#[from] crate::ledger::tokens::TokenOverflow),
    /// A dialect's inclusive input counter was below the cache reads and writes it includes.
    /// The Codex adapter excludes such usage with a diagnostic before it normalizes, so
    /// this reports a broken invariant rather than inconsistent input.
    #[error(transparent)]
    Input(#[from] crate::ledger::tokens::InputBelowCache),
    /// A source-decoding worker thread panicked.
    #[error(transparent)]
    Worker(#[from] crate::sources::parallel::ParallelReadError),
}

impl From<CapacityError> for AdapterError {
    fn from(error: CapacityError) -> Self {
        match error {
            CapacityError::Rows { observations, maximum, limit, .. } => {
                Self::Reconcile(ReconcileError::CapacityExceeded { observations, maximum, limit })
            }
            error @ (CapacityError::Memory { .. } | CapacityError::BelowFloor { .. }) => {
                Self::Capacity(error)
            }
        }
    }
}

/// How one agent's sources decode under process-wide admission (scalable-ingestion plan,
/// "Admission Determinism").
#[derive(Clone, Copy, Debug)]
pub struct Decoding<'a> {
    /// The invocation's ledger: it charges the worker slots, every read, and each source's
    /// rows, and its row ceiling, when it has one, also bounds reconciliation.
    pub admission: &'a MemoryAdmission,
    /// The configured worker count: `UROLLUP_JOBS`, or the default bound, which is at
    /// most eight, so it changes the slot count only when an explicit count passes eight.
    pub workers: NonZeroUsize,
    /// One worker slot `b` in bytes, the same for both agents ([`worker_slot`]).
    pub slot: u64,
}

impl Decoding<'_> {
    /// The threads decode runs on: `min(workers, slots)`, where the slot count is
    /// `clamp(⌊B / (8 × H × b)⌋, 1, max(8, workers))`. It depends only on the budget, `b`
    /// and an explicit worker count past eight, and fewer threads never change a result.
    pub fn threads(&self) -> NonZeroUsize {
        self.workers.min(self.slots())
    }

    fn slots(&self) -> NonZeroUsize {
        self.admission.worker_slots(self.slot, Some(self.workers))
    }

    /// The decode checkpoint for `agent`, before `sources` decode: charges the worker
    /// slots and the parallel read's work order ([`model::work_order`]), whatever the
    /// thread count, releases the coordinating slot, and returns [`Self::threads`].
    pub(crate) fn begin(&self, agent: Agent, sources: usize) -> Result<NonZeroUsize, AdapterError> {
        let slots = u64::try_from(self.slots().get()).unwrap_or(u64::MAX);
        let sources = u64::try_from(sources).unwrap_or(u64::MAX);
        let estimate = slots.saturating_mul(self.slot).saturating_add(model::work_order(sources));
        self.admission.checkpoint(Phase::Decode(agent), estimate, &[(Hold::Coordinating, 0)])?;
        Ok(self.threads())
    }

    /// Charges a source's rows apart from its threads ([`model::source_rows`]) before the
    /// worker returns its result; a refusal ends the source as admission stopping it.
    pub(crate) fn charge_source_rows(
        &self,
        entry: &ManifestEntry,
        version: Option<&str>,
        path: &std::path::Path,
    ) -> Result<(), AdapterError> {
        use crate::ledger::admission::deep_size::DeepSize as _;
        let version = version.map(|version| u64::try_from(version.len()).unwrap_or(u64::MAX));
        if self.admission.charge(Component::Sources, model::source_rows(entry.heap(), version)) {
            Ok(())
        } else {
            Err(AdapterError::Read { path: path.to_owned(), source: SourceReadError::Stopped })
        }
    }

    /// Charges the dialect versions that source rows copy from another source with the
    /// same source ID, beyond what each source charged for its own version.
    ///
    /// `sources` gives each source's ID and declared version in manifest order. `normalize`
    /// gives every source row of an ID the version of its first source that declared one,
    /// so a source with no version, or a shorter one, holds a longer copy than its worker
    /// charged; two roots holding the same transcript share an ID. Called after the join,
    /// before the rows are built.
    pub(crate) fn charge_shared_versions<'s>(
        &self,
        sources: impl Iterator<Item = (Option<&'s AnalyticalId>, Option<&'s str>)> + Clone,
    ) -> Result<(), AdapterError> {
        let mut first: BTreeMap<&AnalyticalId, &str> = BTreeMap::new();
        for (id, version) in sources.clone() {
            if let (Some(id), Some(version)) = (id, version) {
                first.entry(id).or_insert(version);
            }
        }
        let cost = |version: Option<&str>| {
            version.map_or(0, |version| {
                model::allocation(u64::try_from(version.len()).unwrap_or(u64::MAX))
            })
        };
        let extra = sources
            .map(|(id, own)| {
                let copied = id.and_then(|id| first.get(id).copied());
                cost(copied).saturating_sub(cost(own))
            })
            .fold(0_u64, u64::saturating_add);
        if extra == 0 || self.admission.charge(Component::Sources, extra) {
            return Ok(());
        }
        // A refused charge records its refusal before it returns.
        Err(self.admission.refusal().map_or(
            AdapterError::Read { path: PathBuf::new(), source: SourceReadError::Stopped },
            AdapterError::from,
        ))
    }

    /// The row ceiling reconciliation checks again, or none.
    pub(crate) fn row_ceiling(&self) -> crate::ledger::capacity::ObservationCapacity {
        self.admission
            .row_ceiling()
            .cloned()
            .unwrap_or_else(|| crate::ledger::capacity::ObservationCapacity::from_rows(usize::MAX))
    }
}

/// The worker slot `b` for an invocation whose discovery found `discoveries`: the plain
/// slot, plus the zstd decoder when any source has a `.zst` file and the gzip decoder when
/// any has a `.gz` file ([`model::worker_slot`]). Both agents' slots are this size.
pub fn worker_slot(discoveries: &[&Discovery]) -> u64 {
    let has = |representation: Representation| {
        discoveries.iter().any(|discovery| {
            discovery
                .sources
                .iter()
                .any(|source| source.files.files().any(|(_, held)| held == representation))
        })
    };
    model::worker_slot(has(Representation::Zstd), has(Representation::Gzip))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    use super::{Decoding, worker_slot};
    use crate::ledger::admission::model::allocation;
    use crate::ledger::admission::{
        CapacityError, Hold, MemoryAdmission, Phase, ProcessModel, model,
    };
    use crate::ledger::capacity::MemoryBudget;
    use crate::selection::Agent;
    use crate::sources::manifest::Representation;
    use crate::sources::reader::LogicalSource;
    use crate::sources::roots::{DiscoveredSource, Discovery};

    fn workers(count: usize) -> NonZeroUsize {
        NonZeroUsize::new(count).unwrap()
    }

    fn discovery(representations: &[Representation]) -> Discovery {
        let mut discovery = Discovery::default();
        for (index, representation) in representations.iter().enumerate() {
            let mut files = LogicalSource::default();
            files.insert(
                PathBuf::from(format!("s{index}{}", representation.suffix())),
                *representation,
            );
            discovery.sources.push(DiscoveredSource {
                root: PathBuf::from("root"),
                locator: format!("s{index}"),
                files,
            });
        }
        discovery
    }

    #[test]
    fn the_slot_carries_the_decoders_discovery_found_in_either_agent() {
        let plain = discovery(&[Representation::Plain]);
        let zstd = discovery(&[Representation::Plain, Representation::Zstd]);
        let gzip = discovery(&[Representation::Gzip]);
        assert_eq!(worker_slot(&[]), model::worker_slot(false, false));
        assert_eq!(worker_slot(&[&plain]), model::worker_slot(false, false));
        assert_eq!(worker_slot(&[&plain, &zstd]), model::worker_slot(true, false));
        assert_eq!(worker_slot(&[&gzip, &zstd]), model::worker_slot(true, true));
    }

    #[test]
    fn the_decode_checkpoint_charges_the_slots_and_releases_the_coordinating_slot() {
        let slot = model::worker_slot(false, false);
        // ⌊B / (8 × 1.5 × b)⌋ = 2 slots, so eight workers decode on two threads.
        let admission = MemoryAdmission::new(
            MemoryBudget::exact(24 * slot + slot, "test"),
            ProcessModel::DEFAULT,
        );
        admission.raise_hold(Hold::Coordinating, slot).unwrap();
        admission.raise_hold(Hold::Coordinating, slot / 2).unwrap();
        assert_eq!(admission.charged(), slot, "a smaller hold keeps the larger");
        let decoding = Decoding { admission: &admission, workers: workers(8), slot };
        assert_eq!(decoding.threads(), workers(2));
        assert_eq!(decoding.begin(Agent::Codex, 1_000).unwrap(), workers(2));
        assert_eq!(
            admission.charged(),
            2 * slot + model::work_order(1_000),
            "the slots and the work order replace the coordinating slot"
        );
        // The work order is charged whatever the thread count.
        assert!(model::work_order(1_000) >= 3 * allocation(8_000));
        // One worker needs only one thread, and without a budget an explicit count past
        // eight decodes on every thread it names.
        let one = Decoding { workers: workers(1), ..decoding };
        assert_eq!(one.begin(Agent::Claude, 1_000).unwrap(), workers(1));
        assert_eq!(admission.charged(), 2 * slot + model::work_order(1_000));
        let unlimited = MemoryAdmission::unlimited();
        let many = Decoding { admission: &unlimited, workers: workers(16), slot };
        assert_eq!(many.begin(Agent::Codex, 0).unwrap(), workers(16));
    }

    #[test]
    fn a_budget_without_room_for_one_slot_refuses_at_the_decode_checkpoint() {
        let slot = model::worker_slot(true, false);
        let budget = ProcessModel::DEFAULT.baseline + slot;
        let admission =
            MemoryAdmission::new(MemoryBudget::exact(budget, "test"), ProcessModel::DEFAULT);
        let decoding = Decoding { admission: &admission, workers: workers(8), slot };
        let error = decoding.begin(Agent::Codex, 0).unwrap_err();
        let super::AdapterError::Capacity(CapacityError::Memory { phase, estimate, .. }) = error
        else {
            unreachable!("expected a memory refusal, got {error}");
        };
        assert_eq!(phase, Phase::Decode(Agent::Codex));
        assert_eq!(
            estimate,
            Some(ProcessModel::DEFAULT.whole_process(slot + model::work_order(0)))
        );
    }
}
