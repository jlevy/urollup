//! Diagnostics: conflicts, estimates and gaps that reconciliation reports instead of
//! resolving silently (design §3.3).

use super::identity::AnalyticalId;
use crate::sources::evidence::EvidenceRef;

/// What a diagnostic reports. Codes are stable tokens for reports and `--strict`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticCode {
    /// Two observations of one record location carry different content, so the source
    /// changed between reads.
    ConflictingReread,
    /// Observations sharing a key disagree on revision-invariant fields, so the key is
    /// ambiguous and they are not merged.
    ConflictingSharedKey,
    /// Several threads each claim proven ownership of one request.
    ConflictingOwners,
    /// Observations of one request name different accounts.
    ConflictingAccounts,
    /// Observations of one request name different served models.
    ConflictingModels,
    /// A revision selector reported that revisions disagree beyond the value it selected.
    RevisionDisagreement,
    /// A request was observed only as copies, whose usage never counts.
    CopyWithoutOriginal,
    /// A request's candidate set has other members, reported as unresolved usage.
    UnresolvedCandidate,
    /// A normalized usage disagrees with its native counters.
    UsageInconsistency,
    /// A cumulative counter decreased and a new counter epoch started.
    CounterReset,
    /// A cumulative counter's sequence skipped values.
    CounterGap,
    /// Claude block observations of one response disagree on usage.
    ClaudeBlockUsageConflict,
    /// Claude's cache-creation total disagrees with its lifetime breakdown.
    ClaudeCacheCreationBreakdownMismatch,
    /// A nested Claude copy has no original observation.
    ClaudeNestedCopyWithoutOriginal,
    /// Distinct observations claim the same native identity key.
    IdentityKeyConflict,
    /// A legacy Codex copied-history boundary had to be inferred.
    CodexCopiedHistoryInferred,
    /// A Codex cumulative counter decreased and opened a new epoch.
    CodexCounterEpochReset,
    /// Codex emitted a compaction estimate instead of measured usage.
    CodexEstimateCompaction,
    /// Codex emitted a context-window-fill estimate instead of measured usage.
    CodexEstimateContextWindowFill,
    /// One Codex rollout was found at more than one location.
    CodexRolloutDuplicateLocation,
    /// A complete source line was malformed.
    MalformedLine,
    /// A source ended with an incomplete pending line.
    PendingTail,
    /// A thread names a parent whose rollout was not discovered.
    ThreadOrphan,
}

impl DiagnosticCode {
    /// The stable token for this code.
    pub const fn token(self) -> &'static str {
        match self {
            Self::ConflictingReread => "conflicting-reread",
            Self::ConflictingSharedKey => "conflicting-shared-key",
            Self::ConflictingOwners => "conflicting-owners",
            Self::ConflictingAccounts => "conflicting-accounts",
            Self::ConflictingModels => "conflicting-models",
            Self::RevisionDisagreement => "revision-disagreement",
            Self::CopyWithoutOriginal => "copy-without-original",
            Self::UnresolvedCandidate => "unresolved-candidate",
            Self::UsageInconsistency => "usage-inconsistency",
            Self::CounterReset => "counter-reset",
            Self::CounterGap => "counter-gap",
            Self::ClaudeBlockUsageConflict => "claude-block-usage-conflict",
            Self::ClaudeCacheCreationBreakdownMismatch => {
                "claude-cache-creation-breakdown-mismatch"
            }
            Self::ClaudeNestedCopyWithoutOriginal => "claude-nested-copy-without-original",
            Self::IdentityKeyConflict => "identity-key-conflict",
            Self::CodexCopiedHistoryInferred => "codex-copied-history-inferred",
            Self::CodexCounterEpochReset => "codex-counter-epoch-reset",
            Self::CodexEstimateCompaction => "codex-estimate-compaction",
            Self::CodexEstimateContextWindowFill => "codex-estimate-context-window-fill",
            Self::CodexRolloutDuplicateLocation => "codex-rollout-duplicate-location",
            Self::MalformedLine => "malformed-line",
            Self::PendingTail => "pending-tail",
            Self::ThreadOrphan => "thread-orphan",
        }
    }
}

/// One diagnostic with its subject and evidence.
///
/// The derived order is the order a ledger lists diagnostics in, so it never depends on
/// traversal order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Diagnostic {
    /// What is reported.
    pub code: DiagnosticCode,
    /// The entity the diagnostic is about, when it has one.
    pub subject: Option<AnalyticalId>,
    /// The records involved, in canonical order.
    pub evidence: Vec<EvidenceRef>,
    /// A deterministic human-readable detail, built only from sorted data.
    pub detail: String,
}

impl Diagnostic {
    /// A diagnostic whose evidence is sorted and deduplicated.
    pub fn new(
        code: DiagnosticCode,
        subject: Option<AnalyticalId>,
        evidence: impl IntoIterator<Item = EvidenceRef>,
        detail: impl Into<String>,
    ) -> Self {
        let mut evidence: Vec<EvidenceRef> = evidence.into_iter().collect();
        evidence.sort();
        evidence.dedup();
        Self { code, subject, evidence, detail: detail.into() }
    }
}
