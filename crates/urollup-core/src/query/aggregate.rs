//! Aggregation implementation for the public query records.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use crate::accounting::totals::{Completeness, ledger_totals, selection_totals};
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{Counting, Ownership, Request};
use crate::ledger::identity::AnalyticalId;
use crate::ledger::names::Name;
use crate::ledger::tokens::TokenMeasures;
use crate::selection::{IndexedSession, SessionIndex};

use super::{
    AggregateTotals, CoverageSummary, DailyDocument, DailyRow, DiagnosticSummary, GroupBy,
    GroupRow, QueryError, QueryMetadata, QuerySource, REPORT_SCHEMA_VERSION, ReportDocument,
    RequestCounts, ResolvedTimeZone, SessionRow, SessionsDocument, SideTotals, SizeSummary,
    TokenCounts,
};

#[derive(Clone, Copy)]
enum OwnershipClass {
    Owned,
    Ambiguous,
    Unknown,
}

#[derive(Clone, Copy, Debug, Default)]
struct Accumulator {
    requests: RequestCounts,
    tokens: TokenMeasures,
}

impl Accumulator {
    fn add(&mut self, ownership: OwnershipClass, tokens: TokenMeasures) -> Result<(), QueryError> {
        add_request_count(&mut self.requests, ownership)?;
        self.tokens = self.tokens.checked_add(&tokens)?;
        Ok(())
    }

    fn group_row(self, group: GroupBy, value: String) -> Result<GroupRow, QueryError> {
        Ok(GroupRow {
            group: group.token().to_owned(),
            value,
            requests: self.requests,
            tokens: TokenCounts::from_measures(self.tokens)?,
        })
    }
}

/// Builds the full session report over selected threads.
pub fn report(
    sources: &[QuerySource<'_>],
    index: &SessionIndex,
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
    metadata: QueryMetadata,
    groups: &BTreeSet<GroupBy>,
) -> Result<ReportDocument, QueryError> {
    let aggregate = aggregate_totals(sources, selected, all)?;
    let requests = selected_requests(sources, selected, all);
    Ok(ReportDocument {
        schema_version: REPORT_SCHEMA_VERSION,
        query: metadata,
        coverage: coverage_summary(sources, &requests, aggregate.complete)?,
        diagnostics: diagnostic_summaries(sources, selected, all)?,
        breakdowns: breakdowns(&requests, index, groups)?,
        sizes: size_summary(&requests)?,
        totals: aggregate.totals,
    })
}

/// Builds daily calendar rows over selected threads.
pub fn daily(
    sources: &[QuerySource<'_>],
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
    metadata: QueryMetadata,
    timezone: &ResolvedTimeZone,
) -> Result<DailyDocument, QueryError> {
    let requests = selected_requests(sources, selected, all);
    let mut dated: BTreeMap<Option<String>, Accumulator> = BTreeMap::new();
    for selected_request in requests {
        let date = selected_request
            .request
            .last_seen
            .or(selected_request.request.first_seen)
            .map(|timestamp| timezone.zone.to_datetime(timestamp.get()).date().to_string());
        dated.entry(date).or_default().add(
            ownership_class(&selected_request.request.ownership),
            selected_request.measures(),
        )?;
    }
    let mut rows: Vec<_> = dated
        .into_iter()
        .map(|(date, row)| {
            Ok(DailyRow {
                date,
                requests: row.requests,
                tokens: TokenCounts::from_measures(row.tokens)?,
            })
        })
        .collect::<Result<_, QueryError>>()?;
    rows.sort_by(|left, right| match (&left.date, &right.date) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    Ok(DailyDocument {
        schema_version: REPORT_SCHEMA_VERSION,
        query: metadata,
        rows,
        diagnostics: diagnostic_summaries(sources, selected, all)?,
    })
}

/// Builds one row per selected session, plus an explicit unowned row when needed.
pub fn sessions(
    sources: &[QuerySource<'_>],
    index: &SessionIndex,
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
    metadata: QueryMetadata,
) -> Result<SessionsDocument, QueryError> {
    let requests = selected_requests(sources, selected, all);
    let mut by_thread: BTreeMap<Option<AnalyticalId>, Accumulator> =
        selected.iter().cloned().map(|thread| (Some(thread), Accumulator::default())).collect();
    for selected_request in requests {
        let thread = match &selected_request.request.ownership {
            Ownership::Owned { thread } => Some(thread.clone()),
            Ownership::Ambiguous { .. } | Ownership::Unknown => None,
        };
        by_thread.entry(thread).or_default().add(
            ownership_class(&selected_request.request.ownership),
            selected_request.measures(),
        )?;
    }
    let rows = by_thread
        .into_iter()
        .map(|(thread, row)| {
            let indexed = thread.as_ref().and_then(|id| index.get(id));
            Ok(SessionRow {
                thread: thread.map(|id| id.to_string()),
                session: indexed.and_then(IndexedSession::native_id),
                agent: indexed.map_or("unknown", |session| session.agent.token()).to_owned(),
                project: indexed.and_then(|session| session.thread.project.value()).cloned(),
                requests: row.requests,
                tokens: TokenCounts::from_measures(row.tokens)?,
            })
        })
        .collect::<Result<_, QueryError>>()?;
    Ok(SessionsDocument {
        schema_version: REPORT_SCHEMA_VERSION,
        query: metadata,
        rows,
        diagnostics: diagnostic_summaries(sources, selected, all)?,
    })
}

struct AggregatedTotals {
    totals: AggregateTotals,
    complete: bool,
}

fn aggregate_totals(
    sources: &[QuerySource<'_>],
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
) -> Result<AggregatedTotals, QueryError> {
    let mut complete = true;
    let mut requests = RequestCounts::default();
    let mut tokens = TokenMeasures::default();
    let mut unresolved_requests = 0_u64;
    let mut unresolved_tokens = TokenMeasures::default();
    let mut possible_requests = 0_u64;
    let mut possible_tokens = TokenMeasures::default();
    for source in sources {
        if all {
            let source_totals = ledger_totals(&source.ingested.ledger)?;
            complete &= source_totals.completeness == Completeness::Complete;
            requests = add_request_counts(
                requests,
                RequestCounts {
                    owned: source_totals.owned.requests,
                    ambiguous: source_totals.ambiguous.requests,
                    unknown: source_totals.unknown.requests,
                },
            )?;
            tokens = tokens.checked_add(&source_totals.total.tokens)?;
            unresolved_requests = checked_count(
                unresolved_requests,
                source_totals.unresolved.requests,
                "unresolved requests",
            )?;
            unresolved_tokens = unresolved_tokens.checked_add(&source_totals.unresolved.tokens)?;
        } else {
            let source_totals = selection_totals(&source.ingested.ledger, selected)?;
            complete &= source_totals.completeness == Completeness::Complete;
            requests = add_request_counts(
                requests,
                RequestCounts {
                    owned: source_totals.owned.requests,
                    ambiguous: source_totals.ambiguous.requests,
                    unknown: 0,
                },
            )?;
            tokens = tokens.checked_add(&source_totals.counted.tokens)?;
            possible_requests = checked_count(
                possible_requests,
                source_totals.possible.requests,
                "possible requests",
            )?;
            possible_tokens = possible_tokens.checked_add(&source_totals.possible.tokens)?;
        }
    }
    let totals = AggregateTotals {
        requests,
        tokens: TokenCounts::from_measures(tokens)?,
        unresolved: SideTotals {
            requests: unresolved_requests,
            tokens: TokenCounts::from_measures(unresolved_tokens)?,
        },
        possible: SideTotals {
            requests: possible_requests,
            tokens: TokenCounts::from_measures(possible_tokens)?,
        },
    };
    Ok(AggregatedTotals { totals, complete })
}

fn coverage_summary(
    sources: &[QuerySource<'_>],
    requests: &[SelectedRequest<'_>],
    complete: bool,
) -> Result<CoverageSummary, QueryError> {
    let requests_without_usage = usize_count(
        requests.iter().filter(|selected| selected.request.usage.is_none()).count(),
        "requests without usage",
    )?;
    let mut summary =
        CoverageSummary { requests_without_usage, complete, ..CoverageSummary::default() };
    for source in sources {
        summary.copies_excluded = checked_count(
            summary.copies_excluded,
            source.ingested.ledger.coverage.copies,
            "excluded copies",
        )?;
        summary.limit_observations = checked_count(
            summary.limit_observations,
            usize_count(source.ingested.limit_observations.len(), "limit observations")?,
            "limit observations",
        )?;
    }
    Ok(summary)
}

/// One row per diagnostic code over the selected subjects, in code order.
///
/// Diagnostics are filtered by subject first and then aggregated: `count` sums their
/// occurrences and `detail` is the detail of the code's first diagnostic in canonical
/// order, so neither depends on how many sources or ledger rows reported the code.
fn diagnostic_summaries(
    sources: &[QuerySource<'_>],
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
) -> Result<Vec<DiagnosticSummary>, QueryError> {
    let mut by_code: BTreeMap<DiagnosticCode, (u64, &Diagnostic)> = BTreeMap::new();
    let relevant = sources.iter().flat_map(|source| &source.ingested.ledger.diagnostics).filter(
        |diagnostic| {
            all || diagnostic.subject.as_ref().is_none_or(|subject| selected.contains(subject))
        },
    );
    for diagnostic in relevant {
        match by_code.entry(diagnostic.code) {
            Entry::Vacant(entry) => {
                entry.insert((diagnostic.occurrences, diagnostic));
            }
            Entry::Occupied(mut entry) => {
                let (count, first) = entry.get_mut();
                *count = checked_count(*count, diagnostic.occurrences, "diagnostic occurrences")?;
                if diagnostic < *first {
                    *first = diagnostic;
                }
            }
        }
    }
    Ok(by_code
        .into_values()
        .map(|(count, first)| DiagnosticSummary {
            code: first.code.token().to_owned(),
            count,
            detail: first.detail.clone(),
        })
        .collect())
}

struct SelectedRequest<'a> {
    request: &'a Request,
}

impl SelectedRequest<'_> {
    fn measures(&self) -> TokenMeasures {
        self.request
            .usage
            .as_ref()
            .map_or_else(TokenMeasures::default, |usage| usage.revision.usage.into())
    }
}

fn selected_requests<'a>(
    sources: &[QuerySource<'a>],
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
) -> Vec<SelectedRequest<'a>> {
    sources
        .iter()
        .flat_map(|source| source.ingested.ledger.requests.values())
        .filter(|request| request.counting == Counting::Counted)
        .filter(|request| all || request_is_inside(request, selected))
        .map(|request| SelectedRequest { request })
        .collect()
}

fn request_is_inside(request: &Request, selected: &BTreeSet<AnalyticalId>) -> bool {
    match &request.ownership {
        Ownership::Owned { thread } => selected.contains(thread),
        Ownership::Ambiguous { candidates } => {
            !candidates.is_empty() && candidates.iter().all(|thread| selected.contains(thread))
        }
        Ownership::Unknown => false,
    }
}

fn ownership_class(ownership: &Ownership) -> OwnershipClass {
    match ownership {
        Ownership::Owned { .. } => OwnershipClass::Owned,
        Ownership::Ambiguous { .. } => OwnershipClass::Ambiguous,
        Ownership::Unknown => OwnershipClass::Unknown,
    }
}

fn breakdowns(
    requests: &[SelectedRequest<'_>],
    index: &SessionIndex,
    groups: &BTreeSet<GroupBy>,
) -> Result<BTreeMap<String, Vec<GroupRow>>, QueryError> {
    let requested = if groups.is_empty() {
        BTreeSet::from([GroupBy::Project, GroupBy::Account, GroupBy::Model, GroupBy::Effort])
    } else {
        groups.clone()
    };
    let mut result = BTreeMap::new();
    for group in requested {
        let mut rows: BTreeMap<String, Accumulator> = BTreeMap::new();
        for selected in requests {
            let ownership = ownership_class(&selected.request.ownership);
            if group == GroupBy::Model {
                if let Some(usage) = &selected.request.usage {
                    if !usage.revision.model_usage.is_empty() {
                        for component in &usage.revision.model_usage {
                            let value = component
                                .model
                                .as_ref()
                                .map_or("unknown", |model| model.name.as_str())
                                .to_owned();
                            rows.entry(value)
                                .or_default()
                                .add(ownership, component.usage.into())?;
                        }
                        continue;
                    }
                }
            }
            let value = group_value(group, selected.request, index);
            rows.entry(value).or_default().add(ownership, selected.measures())?;
        }
        let rows = rows
            .into_iter()
            .map(|(value, row)| row.group_row(group, value))
            .collect::<Result<_, QueryError>>()?;
        result.insert(group.token().to_owned(), rows);
    }
    Ok(result)
}

fn group_value(group: GroupBy, request: &Request, index: &SessionIndex) -> String {
    match group {
        GroupBy::Project => project_for_owners(request, index),
        // No adapter records a stable account identifier yet.
        GroupBy::Account => "unknown".to_owned(),
        GroupBy::Model => {
            request.model.as_ref().map_or("unknown", |model| model.name.as_str()).to_owned()
        }
        GroupBy::Effort => request.effort.map_or("unknown", Name::as_str).to_owned(),
    }
}

fn project_for_owners(request: &Request, index: &SessionIndex) -> String {
    let owners: Vec<_> = match &request.ownership {
        Ownership::Owned { thread } => vec![thread],
        Ownership::Ambiguous { candidates } => candidates.iter().collect(),
        Ownership::Unknown => return "unknown".to_owned(),
    };
    let values: BTreeSet<_> = owners
        .into_iter()
        .map(|owner| {
            index
                .get(owner)
                .and_then(|session| session.thread.project.value())
                .map_or("unknown", String::as_str)
        })
        .collect();
    if values.len() == 1 {
        values.into_iter().next().unwrap_or("unknown").to_owned()
    } else {
        "ambiguous".to_owned()
    }
}

fn size_summary(requests: &[SelectedRequest<'_>]) -> Result<SizeSummary, QueryError> {
    let mut values: Vec<_> = requests
        .iter()
        .filter_map(|request| {
            request.request.usage.as_ref().and_then(|usage| {
                TokenMeasures::from(usage.revision.usage).inclusive_input().ok().flatten()
            })
        })
        .collect();
    values.sort_unstable();
    Ok(SizeSummary {
        count: usize_count(values.len(), "request sizes")?,
        p50: percentile(&values, 50),
        p90: percentile(&values, 90),
        p99: percentile(&values, 99),
        max: values.last().copied(),
    })
}

fn percentile(values: &[u64], percent: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let rank = values.len().saturating_mul(percent).div_ceil(100).saturating_sub(1);
    values.get(rank).copied()
}

fn add_request_count(
    counts: &mut RequestCounts,
    ownership: OwnershipClass,
) -> Result<(), QueryError> {
    let counter = match ownership {
        OwnershipClass::Owned => &mut counts.owned,
        OwnershipClass::Ambiguous => &mut counts.ambiguous,
        OwnershipClass::Unknown => &mut counts.unknown,
    };
    *counter = counter.checked_add(1).ok_or(QueryError::CountOverflow("requests"))?;
    Ok(())
}

fn add_request_counts(
    left: RequestCounts,
    right: RequestCounts,
) -> Result<RequestCounts, QueryError> {
    Ok(RequestCounts {
        owned: checked_count(left.owned, right.owned, "owned requests")?,
        ambiguous: checked_count(left.ambiguous, right.ambiguous, "ambiguous requests")?,
        unknown: checked_count(left.unknown, right.unknown, "unknown requests")?,
    })
}

fn checked_count(left: u64, right: u64, category: &'static str) -> Result<u64, QueryError> {
    left.checked_add(right).ok_or(QueryError::CountOverflow(category))
}

fn usize_count(value: usize, category: &'static str) -> Result<u64, QueryError> {
    u64::try_from(value).map_err(|_| QueryError::CountOverflow(category))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use super::{daily, report, sessions};
    use crate::adapters::{claude_project, codex_rollout};
    use crate::ledger::diagnostics::DiagnosticCode;
    use crate::query::{GroupBy, QueryMetadata, QuerySource, RequestCounts, ResolvedTimeZone};
    use crate::selection::{Agent, Scope, SelectionQuery, SessionIndex};

    fn fixture(case: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude-project").join(case)
    }

    fn observation(
        offset: u64,
        owner: &str,
        with_usage: bool,
    ) -> crate::ledger::reconcile::RequestObservation {
        use crate::ledger::identity::{IdPrefix, IdentityKey, KeyComponent};
        use crate::ledger::reconcile::{OwnerEvidence, RequestObservation};
        use crate::ledger::tokens::{TokenMeasures, TokenUsage};
        use crate::sources::evidence::EvidenceRef;

        let identity = |prefix, name| {
            IdentityKey::new(prefix, "coverage-test", vec![KeyComponent::text(name)])
                .derive_id()
                .unwrap()
        };
        let mut observation = RequestObservation::new(
            EvidenceRef { source: identity(IdPrefix::Source, "source"), offset, length: 1 },
            "test",
        );
        observation.owner = OwnerEvidence::Proven(identity(IdPrefix::Thread, owner));
        observation.usage = with_usage.then(|| TokenUsage {
            measures: TokenMeasures {
                uncached_input: Some(10),
                output: Some(1),
                ..TokenMeasures::default()
            },
            native: std::collections::BTreeMap::default(),
        });
        observation
    }

    fn coverage_report(
        input: crate::ledger::reconcile::ReconcileInput,
        selected: &BTreeSet<crate::ledger::identity::AnalyticalId>,
        all: bool,
    ) -> crate::query::ReportDocument {
        use crate::adapters::Ingested;
        use crate::ledger::reconcile::{LatestRevision, reconcile};
        let ingested =
            Ingested { ledger: reconcile(input, &LatestRevision).unwrap(), ..Ingested::default() };
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).unwrap();
        report(
            &[QuerySource { agent: Agent::Claude, ingested: &ingested }],
            &SessionIndex::default(),
            selected,
            all,
            QueryMetadata::new("report", "test", Scope::SelfOnly, &timezone),
            &BTreeSet::new(),
        )
        .unwrap()
    }

    #[test]
    fn copy_only_requests_do_not_make_counted_usage_incomplete() {
        use crate::ledger::reconcile::{ObservationRole, ReconcileInput};
        let original = observation(0, "one", true);
        let mut copy = observation(1, "one", true);
        copy.role = ObservationRole::Copy;
        let report = coverage_report(
            ReconcileInput { requests: vec![original, copy], ..ReconcileInput::default() },
            &BTreeSet::new(),
            true,
        );
        assert_eq!(report.totals.requests.owned, 1);
        assert_eq!(report.totals.tokens.uncached_input, Some(10));
        assert_eq!(report.coverage.copies_excluded, 1);
        assert_eq!(report.coverage.requests_without_usage, 0);
        assert!(report.coverage.complete);
        assert!(
            report.diagnostics.iter().any(|diagnostic| diagnostic.code == "copy-without-original")
        );
    }

    #[test]
    fn missing_usage_is_counted_only_inside_the_report_selection() {
        use crate::ledger::reconcile::{OwnerEvidence, ReconcileInput};
        let healthy = observation(0, "one", true);
        let OwnerEvidence::Proven(owner) = healthy.owner.clone() else { unreachable!() };
        let input = ReconcileInput {
            requests: vec![healthy, observation(1, "two", false)],
            ..ReconcileInput::default()
        };
        let all = coverage_report(input.clone(), &BTreeSet::new(), true);
        assert_eq!(all.coverage.requests_without_usage, 1);
        assert!(!all.coverage.complete);
        let selected = coverage_report(input, &BTreeSet::from([owner]), false);
        assert_eq!(selected.totals.requests.owned, 1);
        assert_eq!(selected.coverage.requests_without_usage, 0);
        assert!(selected.coverage.complete);
    }

    #[test]
    fn selected_completeness_respects_gap_scope_and_unresolved_usage() {
        use crate::ledger::coverage::{CoverageGap, UnobservedReason};
        use crate::ledger::reconcile::{OwnerEvidence, ReconcileInput};
        let first = observation(0, "one", true);
        let second = observation(1, "two", true);
        let OwnerEvidence::Proven(first_owner) = first.owner.clone() else { unreachable!() };
        let OwnerEvidence::Proven(second_owner) = second.owner.clone() else { unreachable!() };
        let input = ReconcileInput {
            requests: vec![first.clone(), second],
            gaps: vec![CoverageGap {
                reason: UnobservedReason::EphemeralThread,
                thread: Some(second_owner.clone()),
                evidence: vec![],
            }],
            ..ReconcileInput::default()
        };
        assert!(
            coverage_report(input.clone(), &BTreeSet::from([first_owner.clone()]), false)
                .coverage
                .complete
        );
        assert!(!coverage_report(input, &BTreeSet::from([second_owner]), false).coverage.complete);
        let mut one = first;
        let mut two = observation(2, "one", true);
        one.candidate_tokens.insert("possible-duplicate".to_owned());
        two.candidate_tokens.insert("possible-duplicate".to_owned());
        let unresolved = coverage_report(
            ReconcileInput { requests: vec![one, two], ..ReconcileInput::default() },
            &BTreeSet::from([first_owner]),
            false,
        );
        assert_eq!(unresolved.coverage.requests_without_usage, 0);
        assert!(!unresolved.coverage.complete);
    }

    #[test]
    fn fixture_reports_share_one_reconciled_total() {
        let ingested = claude_project::ingest_root(&fixture("block-record-selection"))
            .expect("fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Claude, &ingested).expect("fixture indexes");
        let selected = index
            .select(&SelectionQuery { all: true, ..SelectionQuery::default() })
            .expect("all sessions select");
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("UTC resolves");
        let source = [QuerySource { agent: Agent::Claude, ingested: &ingested }];
        let report = report(
            &source,
            &index,
            &selected,
            true,
            QueryMetadata::new("report", "all", Scope::SelfOnly, &timezone),
            &BTreeSet::default(),
        )
        .expect("report builds");
        let daily = daily(
            &source,
            &selected,
            true,
            QueryMetadata::new("daily", "all", Scope::SelfOnly, &timezone),
            &timezone,
        )
        .expect("daily builds");
        let sessions = sessions(
            &source,
            &index,
            &selected,
            true,
            QueryMetadata::new("sessions", "all", Scope::SelfOnly, &timezone),
        )
        .expect("sessions build");

        assert_eq!(report.totals.requests, RequestCounts { owned: 3, ..RequestCounts::default() });
        assert_eq!(report.totals.tokens.uncached_input, Some(7));
        assert_eq!(report.totals.tokens.cache_read, Some(93_300));
        assert_eq!(report.totals.tokens.cache_write, Some(1_210));
        assert_eq!(report.totals.tokens.output, Some(470));
        assert_eq!(daily.rows.len(), 1);
        assert_eq!(daily.rows[0].requests, report.totals.requests);
        assert_eq!(sessions.rows.len(), 1);
        assert_eq!(sessions.rows[0].requests, report.totals.requests);
        assert_eq!(
            sessions.rows[0].session.as_deref(),
            Some("00000000-0000-4000-8000-000200000001")
        );
        assert_eq!(report.breakdowns.len(), 4);
        assert!(report.breakdowns.contains_key(GroupBy::Model.token()));
    }

    #[test]
    fn diagnostics_filter_by_selected_subject_then_aggregate_per_code() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/codex-rollout/legacy-subagent-prefix");
        let ingested = codex_rollout::ingest_root(&root).expect("fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Codex, &ingested).expect("fixture indexes");
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("UTC resolves");
        let source = [QuerySource { agent: Agent::Codex, ingested: &ingested }];
        let summaries = |selected: &BTreeSet<_>, all| {
            sessions(
                &source,
                &index,
                selected,
                all,
                QueryMetadata::new("sessions", "test", Scope::SelfOnly, &timezone),
            )
            .expect("sessions build")
            .diagnostics
            .into_iter()
            .map(|row| (row.code, row.count))
            .collect::<Vec<_>>()
        };

        let everything = summaries(&BTreeSet::new(), true);
        assert_eq!(
            everything,
            [("codex-copied-history-inferred".to_owned(), 17), ("thread-orphan".to_owned(), 2)],
            "one row per code, with occurrences summed across ledger rows"
        );

        let subject = ingested
            .ledger
            .diagnostics
            .iter()
            .find(|d| d.code == DiagnosticCode::ThreadOrphan)
            .and_then(|d| d.subject.clone())
            .expect("an orphan names its thread");
        let selected = BTreeSet::from([subject.clone()]);
        let expected: u64 = ingested
            .ledger
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::CodexCopiedHistoryInferred)
            .filter(|d| d.subject.as_ref().is_none_or(|s| *s == subject))
            .map(|d| d.occurrences)
            .sum();
        assert!(expected < 17, "the selection excludes other threads' diagnostics");
        assert_eq!(
            summaries(&selected, false),
            [
                ("codex-copied-history-inferred".to_owned(), expected),
                ("thread-orphan".to_owned(), 1)
            ]
        );
    }
}
