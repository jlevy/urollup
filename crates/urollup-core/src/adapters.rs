//! Persistent agent-log dialect adapters.
//!
//! Adapters turn source snapshots into the normalized entities and reconciled request
//! ledger shared by every report. They preserve source evidence and keep discovery and
//! decoding separate from selection and accounting.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::ledger::entities::{ProviderLimitObservation, Relationship, SourceArtifact, Thread};
use crate::ledger::identity::AnalyticalId;
use crate::ledger::reconcile::{Ledger, ReconcileError};
use crate::sources::manifest::SnapshotManifest;
use crate::sources::reader::SourceReadError;

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
    /// Normalized token arithmetic overflowed.
    #[error(transparent)]
    Tokens(#[from] crate::ledger::tokens::TokenOverflow),
    /// A dialect's inclusive input counter was below its cache-read subset.
    #[error(transparent)]
    Input(#[from] crate::ledger::tokens::InputBelowCacheRead),
    /// A source-decoding worker thread panicked.
    #[error(transparent)]
    Worker(#[from] crate::sources::parallel::ParallelReadError),
}
