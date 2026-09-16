//! Aggregation implementation for the public query records.

use std::collections::{BTreeMap, BTreeSet};

use crate::accounting::totals::{ledger_totals, selection_totals};
use crate::ledger::entities::{AccountAttribution, Counting, Ownership, Request};
use crate::ledger::identity::AnalyticalId;
use crate::ledger::tokens::TokenMeasures;
use crate::selection::SessionIndex;

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
    let totals = aggregate_totals(sources, selected, all)?;
    let requests = selected_requests(sources, selected, all);
    Ok(ReportDocument {
        schema_version: REPORT_SCHEMA_VERSION,
        query: metadata,
        coverage: coverage_summary(sources, &totals)?,
        diagnostics: diagnostic_summaries(sources, selected, all),
        breakdowns: breakdowns(&requests, index, groups)?,
        sizes: size_summary(&requests)?,
        totals,
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
            .map(|timestamp| timezone.zone.to_datetime(timestamp).date().to_string());
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
        diagnostics: diagnostic_summaries(sources, selected, all),
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
        diagnostics: diagnostic_summaries(sources, selected, all),
    })
}

fn aggregate_totals(
    sources: &[QuerySource<'_>],
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
) -> Result<AggregateTotals, QueryError> {
    let mut requests = RequestCounts::default();
    let mut tokens = TokenMeasures::default();
    let mut unresolved_requests = 0_u64;
    let mut unresolved_tokens = TokenMeasures::default();
    let mut possible_requests = 0_u64;
    let mut possible_tokens = TokenMeasures::default();
    for source in sources {
        if all {
            let source_totals = ledger_totals(&source.ingested.ledger)?;
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
    Ok(AggregateTotals {
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
    })
}

fn coverage_summary(
    sources: &[QuerySource<'_>],
    totals: &AggregateTotals,
) -> Result<CoverageSummary, QueryError> {
    let mut summary = CoverageSummary { complete: true, ..CoverageSummary::default() };
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
        summary.requests_without_usage = checked_count(
            summary.requests_without_usage,
            source.ingested.ledger.coverage.requests_without_usage,
            "requests without usage",
        )?;
        if !source.ingested.ledger.gaps.is_empty() {
            summary.complete = false;
        }
    }
    if totals.unresolved.requests > 0
        || totals.possible.requests > 0
        || summary.requests_without_usage > 0
    {
        summary.complete = false;
    }
    Ok(summary)
}

fn diagnostic_summaries(
    sources: &[QuerySource<'_>],
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
) -> Vec<DiagnosticSummary> {
    sources
        .iter()
        .flat_map(|source| &source.ingested.ledger.diagnostics)
        .filter(|diagnostic| {
            all || diagnostic.subject.as_ref().is_none_or(|subject| selected.contains(subject))
        })
        .map(|diagnostic| DiagnosticSummary {
            code: diagnostic.code.token().to_owned(),
            count: diagnostic.occurrences,
            detail: diagnostic.detail.clone(),
        })
        .collect()
}

struct SelectedRequest<'a> {
    request: &'a Request,
}

impl SelectedRequest<'_> {
    fn measures(&self) -> TokenMeasures {
        self.request
            .usage
            .as_ref()
            .map_or_else(TokenMeasures::default, |usage| usage.revision.usage.measures)
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
                                .add(ownership, component.usage.measures)?;
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
        GroupBy::Account => match &request.account {
            AccountAttribution::Attributed(account) => account.clone(),
            AccountAttribution::Conflicting(_) => "conflicting".to_owned(),
            AccountAttribution::Unknown => "unknown".to_owned(),
        },
        GroupBy::Model => {
            request.model.as_ref().map_or("unknown", |model| model.name.as_str()).to_owned()
        }
        GroupBy::Effort => request.effort.clone().unwrap_or_else(|| "unknown".to_owned()),
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
    let mut values: Vec<_> =
        requests
            .iter()
            .filter_map(|request| {
                request.request.usage.as_ref().and_then(|usage| {
                    usage.revision.usage.measures.inclusive_input().ok().flatten()
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
    use crate::adapters::claude_project;
    use crate::query::{GroupBy, QueryMetadata, QuerySource, RequestCounts, ResolvedTimeZone};
    use crate::selection::{Agent, Scope, SelectionQuery, SessionIndex};

    fn fixture(case: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude-project").join(case)
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
        assert_eq!(report.breakdowns.len(), 4);
        assert!(report.breakdowns.contains_key(GroupBy::Model.token()));
    }
}
