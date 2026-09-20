//! Coverage: what reconciliation saw, merged, left unresolved or could not observe
//! (design §2.1, §3.3).

use super::identity::AnalyticalId;
use crate::sources::evidence::EvidenceRef;

/// Why usage never reached local logs.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UnobservedReason {
    /// A Codex `--ephemeral` thread writes no rollout.
    EphemeralThread,
    /// A parallel guardian review's usage is not written.
    ParallelGuardianReview,
    /// Legacy remote compaction consumed usage the rollout does not record.
    LegacyRemoteCompaction,
    /// Another reason, named by a registry token.
    Other(String),
}

/// Usage known to exist but not observed: reported as a gap, never as zero.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CoverageGap {
    /// Why the usage is unobserved.
    pub reason: UnobservedReason,
    /// The thread the gap belongs to, when known.
    pub thread: Option<AnalyticalId>,
    /// The records that reveal the gap, such as a spawn record for an unwritten thread.
    pub evidence: Vec<EvidenceRef>,
}

/// Counters describing one reconciliation.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ReconcileCoverage {
    /// Request observations received.
    pub observations: u64,
    /// Observations dropped as exact re-reads of a record already seen.
    pub rereads: u64,
    /// Observations of a record location whose content changed between reads.
    pub conflicting_rereads: u64,
    /// Copy observations, recorded as evidence only.
    pub copies: u64,
    /// Logical requests in the ledger.
    pub requests: u64,
    /// Requests observed only as copies.
    pub copy_only_requests: u64,
    /// Requests without any usage revision.
    pub requests_without_usage: u64,
    /// Shared keys split because their observations disagree.
    pub conflicting_keys: u64,
    /// Candidate sets with more than one member.
    pub candidate_sets: u64,
    /// Requests reported as unresolved candidates.
    pub unresolved_requests: u64,
}
