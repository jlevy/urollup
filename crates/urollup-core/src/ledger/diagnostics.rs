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
/// traversal order. A ledger holds one diagnostic per code and subject; see [`compact`].
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Diagnostic {
    /// What is reported.
    pub code: DiagnosticCode,
    /// The entity the diagnostic is about, when it has one.
    pub subject: Option<AnalyticalId>,
    /// The records involved, in canonical order.
    ///
    /// A ledger keeps at most [`SAMPLE_EVIDENCE_LIMIT`] of them as samples; see
    /// [`compact`].
    pub evidence: Vec<EvidenceRef>,
    /// How many source records or locations triggered this diagnostic.
    ///
    /// This can exceed the number of distinct evidence references when the same logical
    /// source was discovered at several physical locations.
    pub occurrences: u64,
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
        let occurrences = u64::try_from(evidence.len()).unwrap_or(u64::MAX).max(1);
        Self { code, subject, evidence, occurrences, detail: detail.into() }
    }

    /// Records a larger physical occurrence count than the distinct evidence permits.
    #[must_use]
    pub fn with_occurrences(mut self, occurrences: u64) -> Self {
        self.occurrences = self.occurrences.max(occurrences);
        self
    }
}

/// The most evidence references a compacted diagnostic keeps as samples.
pub const SAMPLE_EVIDENCE_LIMIT: usize = 3;

/// Compacts diagnostics to one per code and subject, in canonical order.
///
/// Identical diagnostics are one report made twice and count once. Each remaining
/// `(code, subject)` group sums its occurrences, keeps the detail of its first diagnostic
/// in canonical order, and keeps at most [`SAMPLE_EVIDENCE_LIMIT`] evidence references,
/// the first in canonical order. The result never depends on input order.
pub fn compact(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort();
    diagnostics.dedup();
    let mut compacted: Vec<Diagnostic> = Vec::new();
    for mut diagnostic in diagnostics {
        diagnostic.evidence.sort();
        diagnostic.evidence.dedup();
        match compacted.last_mut() {
            Some(group) if group.code == diagnostic.code && group.subject == diagnostic.subject => {
                group.occurrences = group.occurrences.saturating_add(diagnostic.occurrences);
                group.evidence.append(&mut diagnostic.evidence);
                group.evidence.sort();
                group.evidence.dedup();
                group.evidence.truncate(SAMPLE_EVIDENCE_LIMIT);
            }
            _ => {
                diagnostic.evidence.truncate(SAMPLE_EVIDENCE_LIMIT);
                compacted.push(diagnostic);
            }
        }
    }
    compacted
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticCode, SAMPLE_EVIDENCE_LIMIT, compact};
    use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent};
    use crate::sources::evidence::EvidenceRef;

    fn id(prefix: IdPrefix, n: i64) -> AnalyticalId {
        IdentityKey::new(prefix, "test-diagnostic", vec![KeyComponent::Integer(n)])
            .derive_id()
            .expect("test ID derives")
    }

    fn evidence(offset: u64) -> EvidenceRef {
        EvidenceRef { source: id(IdPrefix::Source, 1), offset, length: 10 }
    }

    fn diagnostic(
        code: DiagnosticCode,
        subject: Option<i64>,
        offsets: &[u64],
        detail: &str,
    ) -> Diagnostic {
        Diagnostic::new(
            code,
            subject.map(|n| id(IdPrefix::Thread, n)),
            offsets.iter().copied().map(evidence),
            detail,
        )
    }

    #[test]
    fn compaction_sums_occurrences_per_code_and_subject() {
        let input = vec![
            diagnostic(DiagnosticCode::MalformedLine, None, &[40, 50], "b detail"),
            diagnostic(DiagnosticCode::MalformedLine, None, &[10, 20, 30], "a detail"),
            diagnostic(DiagnosticCode::MalformedLine, None, &[10, 20, 30], "a detail"),
            diagnostic(DiagnosticCode::ThreadOrphan, Some(1), &[5], "orphan"),
            diagnostic(DiagnosticCode::ThreadOrphan, Some(2), &[6], "orphan"),
            diagnostic(DiagnosticCode::PendingTail, None, &[], "tail").with_occurrences(4),
        ];
        let mut reversed = input.clone();
        reversed.reverse();

        let compacted = compact(input);
        assert_eq!(compact(reversed), compacted, "input order never matters");
        let rows: Vec<_> = compacted
            .iter()
            .map(|d| (d.code, d.subject.is_some(), d.occurrences, d.detail.as_str()))
            .collect();
        assert_eq!(
            rows,
            [
                (DiagnosticCode::MalformedLine, false, 5, "a detail"),
                (DiagnosticCode::PendingTail, false, 4, "tail"),
                (DiagnosticCode::ThreadOrphan, true, 1, "orphan"),
                (DiagnosticCode::ThreadOrphan, true, 1, "orphan"),
            ],
            "identical reports count once and distinct subjects stay separate"
        );
        let malformed = &compacted[0];
        assert_eq!(malformed.evidence.len(), SAMPLE_EVIDENCE_LIMIT);
        assert_eq!(
            malformed.evidence.iter().map(|e| e.offset).collect::<Vec<_>>(),
            vec![10, 20, 30],
            "samples are the first references in canonical order"
        );
    }
}
