//! Aggregation implementation for the public query records.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::Date;

use crate::accounting::totals::{Completeness, ledger_totals, selection_totals};
use crate::adapters::Ingested;
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{Counting, Ownership, Request};
use crate::ledger::identity::AnalyticalId;
use crate::ledger::names::Name;
use crate::ledger::provider::provider_for;
use crate::ledger::tokens::TokenMeasures;
use crate::selection::{Agent, IndexedSession, SessionIndex};

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
    // The report reads the selected requests three times, so it collects them once.
    let requests: Vec<_> = selected_requests(sources, selected, all).collect();
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
    let mut dated: BTreeMap<Option<String>, Accumulator> = BTreeMap::new();
    for selected_request in selected_requests(sources, selected, all) {
        let date = request_date(selected_request.request, timezone).map(|date| date.to_string());
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

/// The group one `sessions` row reports.
///
/// Ownership and agent are separate facts. A request whose owner is ambiguous or unknown
/// still came from one agent's logs, so it joins that agent's unowned group: never a
/// guessed session or project, and never another agent's requests.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SessionGroup {
    /// An agent's requests with no single proven owner; these sort before every thread.
    Unowned(Agent),
    /// One analytical thread.
    Thread(AnalyticalId),
}

/// One session row's totals, the calendar extent `daily` would show for them, and the
/// agent whose source reported its requests.
#[derive(Clone, Copy, Debug, Default)]
struct SessionAccumulator {
    source: Option<Agent>,
    totals: Accumulator,
    last_date: Option<Date>,
    undated_requests: u64,
}

/// Builds one row per selected session, plus one unowned row per agent whose requests
/// include some with no single proven owner.
pub fn sessions(
    sources: &[QuerySource<'_>],
    index: &SessionIndex,
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
    metadata: QueryMetadata,
    timezone: &ResolvedTimeZone,
) -> Result<SessionsDocument, QueryError> {
    let mut groups: BTreeMap<SessionGroup, SessionAccumulator> = selected
        .iter()
        .cloned()
        .map(|thread| (SessionGroup::Thread(thread), SessionAccumulator::default()))
        .collect();
    // Walk one source at a time, where its agent is in scope, so the requests every
    // command selects need not each carry an agent.
    for source in sources {
        for selected_request in source_requests(source.ingested, selected, all) {
            let ownership = &selected_request.request.ownership;
            let group = match ownership {
                Ownership::Owned { thread } => SessionGroup::Thread(thread.clone()),
                Ownership::Ambiguous { .. } | Ownership::Unknown => {
                    SessionGroup::Unowned(source.agent)
                }
            };
            let row = groups.entry(group).or_default();
            row.source.get_or_insert(source.agent);
            row.totals.add(ownership_class(ownership), selected_request.measures())?;
            match request_date(selected_request.request, timezone) {
                Some(date) => row.last_date = row.last_date.max(Some(date)),
                None => {
                    row.undated_requests =
                        checked_count(row.undated_requests, 1, "undated requests")?;
                }
            }
        }
    }
    let rows = groups
        .into_iter()
        .map(|(group, row)| {
            let (thread, indexed, agent) = match group {
                SessionGroup::Unowned(agent) => (None, None, Some(agent)),
                SessionGroup::Thread(id) => {
                    // The index names a thread's agent; a thread it lacks keeps the agent
                    // of the source that reported its requests.
                    let indexed = index.get(&id);
                    (Some(id), indexed, indexed.map(|session| session.agent).or(row.source))
                }
            };
            Ok(SessionRow {
                thread: thread.map(|id| id.to_string()),
                session: indexed.and_then(IndexedSession::native_id),
                agent: agent.map_or("unknown", Agent::token).to_owned(),
                project: indexed.and_then(|session| session.thread.project.value()).cloned(),
                requests: row.totals.requests,
                tokens: TokenCounts::from_measures(row.totals.tokens)?,
                last_date: row.last_date.map(|date| date.to_string()),
                undated_requests: row.undated_requests,
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

/// Every source's counted requests inside the selection, in source then ledger order.
fn selected_requests<'a>(
    sources: &[QuerySource<'a>],
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
) -> impl Iterator<Item = SelectedRequest<'a>> {
    sources.iter().flat_map(move |source| source_requests(source.ingested, selected, all))
}

/// One source's counted requests inside the selection, in ledger order.
fn source_requests<'a>(
    ingested: &'a Ingested,
    selected: &BTreeSet<AnalyticalId>,
    all: bool,
) -> impl Iterator<Item = SelectedRequest<'a>> {
    ingested
        .ledger
        .requests
        .values()
        .filter(|request| request.counting == Counting::Counted)
        .filter(move |request| all || request_is_inside(request, selected))
        .map(|request| SelectedRequest { request })
}

/// The calendar date `daily` buckets a request under: its last, else its first, timestamp
/// in the report timezone.
fn request_date(request: &Request, timezone: &ResolvedTimeZone) -> Option<Date> {
    request
        .last_seen
        .or(request.first_seen)
        .map(|timestamp| timezone.zone.to_datetime(timestamp.get()).date())
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
            if matches!(group, GroupBy::Model | GroupBy::Provider) {
                if let Some(usage) = &selected.request.usage {
                    if !usage.revision.model_usage.is_empty() {
                        for component in &usage.revision.model_usage {
                            let model = component
                                .model
                                .as_ref()
                                .map_or("unknown", |model| model.name.as_str());
                            let value = if group == GroupBy::Provider {
                                provider_label(selected.request, index, Some(model))
                            } else {
                                model.to_owned()
                            };
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
        GroupBy::Agent => agent_for_owners(request, index),
        GroupBy::Provider => {
            let model = request.model.as_ref().map(|model| model.name.as_str());
            provider_label(request, index, model)
        }
        GroupBy::Purpose => purpose_for_owners(request, index),
    }
}

fn provider_label(request: &Request, index: &SessionIndex, model: Option<&str>) -> String {
    let agent = agent_enum_for_owners(request, index);
    match provider_for(agent, model) {
        crate::ledger::entities::Basis::Observed(token)
        | crate::ledger::entities::Basis::Configured(token)
        | crate::ledger::entities::Basis::Inferred(token) => token.to_owned(),
        crate::ledger::entities::Basis::Unknown => "unknown".to_owned(),
    }
}

fn agent_enum_for_owners(request: &Request, index: &SessionIndex) -> Agent {
    let owners: Vec<_> = match &request.ownership {
        Ownership::Owned { thread } => vec![thread],
        Ownership::Ambiguous { candidates } => candidates.iter().collect(),
        Ownership::Unknown => return Agent::Pi,
    };
    let values: BTreeSet<_> =
        owners.into_iter().map(|owner| index.get(owner).map(|session| session.agent)).collect();
    if values.len() == 1 {
        values.into_iter().next().flatten().unwrap_or(Agent::Pi)
    } else {
        Agent::Pi
    }
}

fn agent_for_owners(request: &Request, index: &SessionIndex) -> String {
    let owners: Vec<_> = match &request.ownership {
        Ownership::Owned { thread } => vec![thread],
        Ownership::Ambiguous { candidates } => candidates.iter().collect(),
        Ownership::Unknown => return "unknown".to_owned(),
    };
    let values: BTreeSet<_> = owners
        .into_iter()
        .map(|owner| index.get(owner).map_or("unknown", |session| session.agent.token()))
        .collect();
    if values.len() == 1 {
        values.into_iter().next().unwrap_or("unknown").to_owned()
    } else {
        "ambiguous".to_owned()
    }
}

fn purpose_for_owners(request: &Request, index: &SessionIndex) -> String {
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
                .and_then(|session| session.thread.purpose.value())
                .map_or("unknown", String::as_str)
        })
        .collect();
    if values.len() == 1 {
        values.into_iter().next().unwrap_or("unknown").to_owned()
    } else {
        "ambiguous".to_owned()
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
        use crate::ledger::tokens::TokenMeasures;
        use crate::sources::evidence::EvidenceRef;

        let identity = |prefix, name| {
            IdentityKey::new(prefix, "coverage-test", vec![KeyComponent::text(name)])
                .derive_id()
                .unwrap()
        };
        let mut observation = RequestObservation::new(EvidenceRef { source: 0, offset, length: 1 });
        observation.owner = OwnerEvidence::Proven(identity(IdPrefix::Thread, owner));
        observation.usage = with_usage.then(|| {
            TokenMeasures { uncached_input: Some(10), output: Some(1), ..TokenMeasures::default() }
                .into()
        });
        observation
    }

    /// Reconciles synthetic observations from one source into an adapter result.
    fn reconciled(
        mut input: crate::ledger::reconcile::ReconcileInput,
    ) -> crate::adapters::Ingested {
        use crate::adapters::Ingested;
        use crate::ledger::identity::{IdPrefix, IdentityKey, KeyComponent};
        use crate::ledger::reconcile::{LatestRevision, reconcile};
        use crate::sources::evidence::SourceTable;
        input.source_table = SourceTable::from_ordered(vec![
            IdentityKey::new(IdPrefix::Source, "coverage-test", vec![KeyComponent::text("source")])
                .derive_id()
                .unwrap(),
        ]);
        Ingested { ledger: reconcile(input, &LatestRevision).unwrap(), ..Ingested::default() }
    }

    /// Los Angeles time from its POSIX rule. A named zone needs a time zone database,
    /// which Windows runners lack (`uro-o3l4`); the rule is exact for 2026.
    fn pacific() -> ResolvedTimeZone {
        ResolvedTimeZone {
            zone: jiff::tz::TimeZone::posix("PST8PDT,M3.2.0,M11.1.0").expect("valid rule"),
            name: "America/Los_Angeles".to_owned(),
        }
    }

    fn coverage_report(
        input: crate::ledger::reconcile::ReconcileInput,
        selected: &BTreeSet<crate::ledger::identity::AnalyticalId>,
        all: bool,
    ) -> crate::query::ReportDocument {
        let ingested = reconciled(input);
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
        use crate::ledger::identity::KeyComponent;
        use crate::ledger::reconcile::{OwnerEvidence, ReconcileInput};
        use crate::ledger::scope::tests::PROVIDER_RESPONSE;
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
        let shared = PROVIDER_RESPONSE
            .key(vec![KeyComponent::text("anthropic"), KeyComponent::text("possible-duplicate")])
            .unwrap()
            .derive()
            .unwrap();
        one.keys.push(shared.clone());
        two.keys.push(shared);
        one.invariants.push(("session", "one".into()));
        two.invariants.push(("session", "two".into()));
        let unresolved = coverage_report(
            ReconcileInput { requests: vec![one, two], ..ReconcileInput::default() },
            &BTreeSet::from([first_owner]),
            false,
        );
        assert_eq!(unresolved.coverage.requests_without_usage, 0);
        assert!(!unresolved.coverage.complete);
    }

    /// One agent's synthetic ledger: an owned request, a request two threads both prove
    /// they own (ambiguous), and a request with no owner evidence (unknown).
    fn ledger_with_unowned_requests(agent: &str) -> crate::adapters::Ingested {
        use crate::ledger::identity::KeyComponent;
        use crate::ledger::reconcile::{OwnerEvidence, ReconcileInput};
        use crate::ledger::scope::tests::PROVIDER_RESPONSE;
        let shared = PROVIDER_RESPONSE
            .key(vec![KeyComponent::text("provider"), KeyComponent::text(agent)])
            .unwrap()
            .derive()
            .unwrap();
        let mut first_claim = observation(1, &format!("{agent}-first"), true);
        first_claim.keys.push(shared.clone());
        let mut second_claim = observation(2, &format!("{agent}-second"), true);
        second_claim.keys.push(shared);
        let mut unknown = observation(3, &format!("{agent}-unused"), true);
        unknown.owner = OwnerEvidence::None;
        reconciled(ReconcileInput {
            requests: vec![
                observation(0, &format!("{agent}-owned"), true),
                first_claim,
                second_claim,
                unknown,
            ],
            ..ReconcileInput::default()
        })
    }

    /// Asserts that session rows partition the report's counted requests and tokens.
    fn assert_rows_reconcile(
        rows: &[crate::query::SessionRow],
        totals: &crate::query::AggregateTotals,
    ) {
        let mut requests = RequestCounts::default();
        let mut tokens = crate::query::TokenCounts::default();
        let add = |sum: &mut Option<u64>, value: Option<u64>| {
            if let Some(value) = value {
                *sum = Some(sum.unwrap_or(0) + value);
            }
        };
        for row in rows {
            requests.owned += row.requests.owned;
            requests.ambiguous += row.requests.ambiguous;
            requests.unknown += row.requests.unknown;
            add(&mut tokens.uncached_input, row.tokens.uncached_input);
            add(&mut tokens.cache_read, row.tokens.cache_read);
            add(&mut tokens.cache_write, row.tokens.cache_write);
            add(&mut tokens.cache_write_5m, row.tokens.cache_write_5m);
            add(&mut tokens.cache_write_1h, row.tokens.cache_write_1h);
            add(&mut tokens.cache_write_unspecified, row.tokens.cache_write_unspecified);
            add(&mut tokens.output, row.tokens.output);
            add(&mut tokens.reasoning, row.tokens.reasoning);
            add(&mut tokens.provider_only, row.tokens.provider_only);
            add(&mut tokens.total, row.tokens.total);
        }
        assert_eq!(requests, totals.requests, "session rows partition the counted requests");
        assert_eq!(tokens, totals.tokens, "session rows partition the counted tokens");
    }

    #[test]
    fn unowned_session_rows_keep_their_source_agent() {
        let claude = ledger_with_unowned_requests("claude");
        let codex = ledger_with_unowned_requests("codex");
        let sources = [
            QuerySource { agent: Agent::Claude, ingested: &claude },
            QuerySource { agent: Agent::Codex, ingested: &codex },
        ];
        let index = SessionIndex::default();
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).unwrap();
        let document = sessions(
            &sources,
            &index,
            &BTreeSet::new(),
            true,
            QueryMetadata::new("sessions", "all", Scope::SelfOnly, &timezone),
            &timezone,
        )
        .unwrap();

        let unowned = RequestCounts { owned: 0, ambiguous: 1, unknown: 1 };
        let owned = RequestCounts { owned: 1, ..RequestCounts::default() };
        let rows: Vec<_> = document
            .rows
            .iter()
            .map(|row| (row.thread.as_deref(), row.agent.as_str(), row.requests))
            .collect();
        assert_eq!(
            rows[..2],
            [(None, "claude", unowned), (None, "codex", unowned)],
            "one unowned row per source agent, before the threads"
        );
        // Threads follow in analytical-ID order, each keeping its source agent.
        let mut threads = rows[2..].to_vec();
        assert!(threads.iter().all(|(thread, ..)| thread.is_some()));
        assert!(threads.is_sorted_by_key(|(thread, ..)| *thread));
        threads.sort_by_key(|(_, agent, _)| *agent);
        assert_eq!(
            threads.iter().map(|(_, agent, requests)| (*agent, *requests)).collect::<Vec<_>>(),
            [("claude", owned), ("codex", owned)],
            "never an unknown agent"
        );
        for row in document.rows.iter().filter(|row| row.thread.is_none()) {
            assert_eq!(
                (&row.session, &row.project),
                (&None, &None),
                "no session or project guessed"
            );
            assert_eq!(row.tokens.uncached_input, Some(20));
            assert_eq!(row.tokens.output, Some(2));
            // No observation has a timestamp, so each agent's row counts only its own
            // two unowned requests as undated.
            assert_eq!((row.last_date.as_deref(), row.undated_requests), (None, 2));
        }

        let report = report(
            &sources,
            &index,
            &BTreeSet::new(),
            true,
            QueryMetadata::new("report", "all", Scope::SelfOnly, &timezone),
            &BTreeSet::new(),
        )
        .unwrap();
        assert_eq!(report.totals.requests, RequestCounts { owned: 2, ambiguous: 2, unknown: 2 });
        assert_rows_reconcile(&document.rows, &report.totals);
    }

    #[test]
    fn mixed_agent_fixtures_keep_unowned_rows_per_agent_and_reconcile() {
        let claude =
            claude_project::ingest_root(&fixture("ambiguous-owner")).expect("Claude ingests");
        let codex = codex_rollout::ingest_root(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/codex-rollout/ambiguous-owner"),
        )
        .expect("Codex ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Claude, &claude).expect("Claude indexes");
        index.add(Agent::Codex, &codex).expect("Codex indexes");
        let selected = index
            .select(&SelectionQuery { all: true, ..SelectionQuery::default() })
            .expect("all sessions select");
        let sources = [
            QuerySource { agent: Agent::Claude, ingested: &claude },
            QuerySource { agent: Agent::Codex, ingested: &codex },
        ];
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("UTC resolves");
        let totals = |sources: &[QuerySource<'_>]| {
            report(
                sources,
                &index,
                &selected,
                true,
                QueryMetadata::new("report", "all", Scope::SelfOnly, &timezone),
                &BTreeSet::new(),
            )
            .expect("report builds")
            .totals
        };
        let document = sessions(
            &sources,
            &index,
            &selected,
            true,
            QueryMetadata::new("sessions", "all", Scope::SelfOnly, &timezone),
            &timezone,
        )
        .expect("sessions build");

        let ambiguous = RequestCounts { ambiguous: 1, ..RequestCounts::default() };
        let unowned: Vec<_> = document
            .rows
            .iter()
            .filter(|row| row.thread.is_none())
            .map(|row| (row.agent.as_str(), row.session.as_deref(), row.requests, row.tokens.total))
            .collect();
        assert_eq!(
            unowned,
            [("claude", None, ambiguous, Some(2_310)), ("codex", None, ambiguous, Some(12_600))]
        );
        // Each agent's unowned row carries the calendar fields `daily` uses.
        for row in document.rows.iter().filter(|row| row.thread.is_none()) {
            assert_eq!((row.last_date.as_deref(), row.undated_requests), (Some("2026-09-05"), 0));
        }
        assert!(document.rows.iter().all(|row| row.agent != "unknown"));

        let combined = totals(&sources);
        assert_eq!(combined.requests, RequestCounts { owned: 4, ambiguous: 2, unknown: 0 });
        assert_eq!(combined.tokens.total, Some(30_335));
        assert_rows_reconcile(&document.rows, &combined);
        for (agent, source) in [("claude", &sources[..1]), ("codex", &sources[1..])] {
            let rows: Vec<_> =
                document.rows.iter().filter(|row| row.agent == agent).cloned().collect();
            assert_rows_reconcile(&rows, &totals(source));
        }
    }

    #[test]
    fn selected_requests_stay_one_pointer_wide() {
        // `report` collects one per selected request; `daily` and `sessions` stream them.
        // Only `sessions` needs the source agent, and it reads that from the source.
        assert_eq!(
            std::mem::size_of::<super::SelectedRequest<'_>>(),
            std::mem::size_of::<&crate::ledger::entities::Request>()
        );
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
            &timezone,
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
                &timezone,
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

    #[test]
    fn session_rows_date_their_latest_request_and_count_undated_ones() {
        use crate::ledger::reconcile::{OwnerEvidence, ReconcileInput, RequestObservation};
        let dated = |offset, owner, instant: &str| {
            let mut observation = observation(offset, owner, true);
            observation.timestamp = Some(instant.parse::<jiff::Timestamp>().unwrap().into());
            observation
        };
        let thread = |observation: &RequestObservation| {
            let OwnerEvidence::Proven(id) = &observation.owner else { unreachable!() };
            id.to_string()
        };
        let requests = vec![
            dated(0, "one", "2026-09-14T12:00:00Z"),
            // 05:00 UTC on the 16th is still the 15th in Los Angeles.
            dated(1, "one", "2026-09-16T05:00:00Z"),
            dated(2, "two", "2026-09-13T12:00:00Z"),
            observation(3, "two", true),
            observation(4, "three", true),
        ];
        let threads = [thread(&requests[0]), thread(&requests[2]), thread(&requests[4])];
        let ingested = reconciled(ReconcileInput { requests, ..ReconcileInput::default() });
        let source = [QuerySource { agent: Agent::Claude, ingested: &ingested }];
        let calendar = |timezone: ResolvedTimeZone| {
            let rows = sessions(
                &source,
                &SessionIndex::default(),
                &BTreeSet::new(),
                true,
                QueryMetadata::new("sessions", "all", Scope::SelfOnly, &timezone),
                &timezone,
            )
            .unwrap()
            .rows;
            threads
                .iter()
                .map(|id| {
                    let row = rows.iter().find(|row| row.thread.as_ref() == Some(id)).unwrap();
                    (row.last_date.clone(), row.undated_requests)
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            calendar(pacific()),
            [(Some("2026-09-15".to_owned()), 0), (Some("2026-09-13".to_owned()), 1), (None, 1)]
        );
        let utc = ResolvedTimeZone::resolve(Some("UTC")).unwrap();
        assert_eq!(calendar(utc)[0], (Some("2026-09-16".to_owned()), 0));
    }

    #[test]
    fn a_request_two_sessions_both_prove_is_dated_on_the_unowned_row() {
        use crate::ledger::identity::KeyComponent;
        use crate::ledger::reconcile::{OwnerEvidence, ReconcileInput, RequestObservation};
        use crate::ledger::scope::tests::PROVIDER_RESPONSE;
        let shared = PROVIDER_RESPONSE
            .key(vec![KeyComponent::text("anthropic"), KeyComponent::text("copied-response")])
            .unwrap()
            .derive()
            .unwrap();
        let claimed = |offset, owner| {
            let mut observation = observation(offset, owner, true);
            observation.timestamp =
                Some("2026-09-14T12:00:00Z".parse::<jiff::Timestamp>().unwrap().into());
            observation.keys.push(shared.clone());
            observation
        };
        let thread = |observation: &RequestObservation| {
            let OwnerEvidence::Proven(id) = &observation.owner else { unreachable!() };
            id.clone()
        };
        let requests = vec![claimed(0, "one"), claimed(1, "two")];
        // Both sessions are selected, as in a whole-history run.
        let selected = BTreeSet::from([thread(&requests[0]), thread(&requests[1])]);
        let ingested = reconciled(ReconcileInput { requests, ..ReconcileInput::default() });
        let source = [QuerySource { agent: Agent::Claude, ingested: &ingested }];
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).unwrap();
        let rows = sessions(
            &source,
            &SessionIndex::default(),
            &selected,
            true,
            QueryMetadata::new("sessions", "all", Scope::SelfOnly, &timezone),
            &timezone,
        )
        .unwrap()
        .rows;

        // The local aggregate counts neither session as stable: each has no owned request.
        for id in &selected {
            let row =
                rows.iter().find(|row| row.thread.as_deref() == Some(&*id.to_string())).unwrap();
            assert_eq!(row.requests, RequestCounts::default());
            assert_eq!((row.last_date.as_deref(), row.undated_requests), (None, 0));
        }
        let unowned = rows.iter().find(|row| row.thread.is_none()).unwrap();
        assert_eq!(unowned.requests, RequestCounts { ambiguous: 1, ..RequestCounts::default() });
        assert_eq!(unowned.last_date.as_deref(), Some("2026-09-14"));
    }

    #[test]
    fn session_dates_match_a_daily_query_over_each_thread() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/codex-rollout/archived-rename");
        let ingested = codex_rollout::ingest_root(&root).expect("fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Codex, &ingested).expect("fixture indexes");
        let selected = index
            .select(&SelectionQuery { all: true, ..SelectionQuery::default() })
            .expect("all sessions select");
        let timezone = pacific();
        let source = [QuerySource { agent: Agent::Codex, ingested: &ingested }];
        let rows = sessions(
            &source,
            &index,
            &selected,
            true,
            QueryMetadata::new("sessions", "all", Scope::SelfOnly, &timezone),
            &timezone,
        )
        .expect("sessions build")
        .rows;

        let mut dated = 0;
        for thread in &selected {
            let days = daily(
                &source,
                &BTreeSet::from([thread.clone()]),
                false,
                QueryMetadata::new("daily", "session", Scope::SelfOnly, &timezone),
                &timezone,
            )
            .expect("daily builds")
            .rows;
            let row = rows
                .iter()
                .find(|row| row.thread.as_deref() == Some(thread.to_string().as_str()))
                .expect("every selected thread has a session row");
            let last = days.iter().filter_map(|day| day.date.clone()).max();
            let undated: u64 = days
                .iter()
                .filter(|day| day.date.is_none())
                .map(|day| day.requests.total().expect("counts fit"))
                .sum();
            assert_eq!((row.last_date.clone(), row.undated_requests), (last, undated));
            dated += usize::from(row.last_date.is_some());
        }
        assert!(dated >= 2, "the fixture has several dated sessions");
    }
}
