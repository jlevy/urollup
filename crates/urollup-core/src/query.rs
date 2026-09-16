//! Shared, deterministic report queries for the CLI and optional serving layer.
//!
//! Adapters and the reconciliation ledger remain the source of truth. This module only
//! selects counted requests, aggregates normalized measures, and returns stable report
//! records. Presentation code can serialize or tabulate them without reimplementing
//! accounting rules.

use std::collections::BTreeMap;

use jiff::tz::TimeZone;
use serde::Serialize;

use crate::adapters::Ingested;
use crate::ledger::tokens::{TokenMeasures, TokenOverflow};
use crate::selection::{Agent, Scope};

mod aggregate;

pub use aggregate::{daily, report, sessions};

/// The JSON schema version of milestone 0.1 report renderings.
pub const REPORT_SCHEMA_VERSION: u8 = 1;

/// One reconciled adapter result available to a query.
#[derive(Clone, Copy, Debug)]
pub struct QuerySource<'a> {
    /// The agent that wrote the source.
    pub agent: Agent,
    /// Its normalized and reconciled records.
    pub ingested: &'a Ingested,
}

/// A normalized timezone used for calendar bucketing and recorded in query output.
#[derive(Clone, Debug)]
pub struct ResolvedTimeZone {
    pub(crate) zone: TimeZone,
    name: String,
}

impl ResolvedTimeZone {
    /// Resolves an IANA timezone, or the system timezone when no name was supplied.
    pub fn resolve(name: Option<&str>) -> Result<Self, QueryError> {
        let zone = match name {
            Some("UTC" | "Etc/UTC" | "Etc/GMT") => TimeZone::UTC,
            Some(name) => TimeZone::get(name)
                .map_err(|source| QueryError::InvalidTimeZone { name: name.to_owned(), source })?,
            None => TimeZone::system(),
        };
        let name = zone.iana_name().unwrap_or("Etc/Unknown").to_owned();
        Ok(Self { zone, name })
    }

    /// The normalized IANA name, or `Etc/Unknown` for a system zone without one.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A report grouping supported in milestone 0.1.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GroupBy {
    /// Logical project from the owning thread.
    Project,
    /// Stable account attribution.
    Account,
    /// Served or requested model, splitting multi-model requests by model usage.
    Model,
    /// Recorded reasoning effort.
    Effort,
}

impl GroupBy {
    /// The stable output token.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Account => "account",
            Self::Model => "model",
            Self::Effort => "effort",
        }
    }
}

/// The selection and calendar policy recorded with every report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryMetadata {
    /// Command that produced the result.
    pub command: String,
    /// Human-readable normalized selector kind.
    pub selection: String,
    /// Effective hierarchy scope.
    pub scope: String,
    /// Normalized IANA timezone name.
    pub timezone: String,
}

impl QueryMetadata {
    /// Builds metadata from already-normalized command inputs.
    pub fn new(
        command: impl Into<String>,
        selection: impl Into<String>,
        scope: Scope,
        timezone: &ResolvedTimeZone,
    ) -> Self {
        Self {
            command: command.into(),
            selection: selection.into(),
            scope: match scope {
                Scope::SelfOnly => "self",
                Scope::Descendants => "descendants",
            }
            .to_owned(),
            timezone: timezone.name().to_owned(),
        }
    }
}

/// Request counts split by ownership status.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct RequestCounts {
    /// Requests with a proven owner.
    pub owned: u64,
    /// Requests with candidate owners.
    pub ambiguous: u64,
    /// Requests with no owner evidence.
    pub unknown: u64,
}

impl RequestCounts {
    /// Total requests represented by the split.
    pub fn total(self) -> Result<u64, QueryError> {
        self.owned
            .checked_add(self.ambiguous)
            .and_then(|value| value.checked_add(self.unknown))
            .ok_or(QueryError::CountOverflow("requests"))
    }
}

/// Normalized disjoint token categories in a report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct TokenCounts {
    /// Input tokens neither read from nor written to a cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncached_input: Option<u64>,
    /// Input tokens read from cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u64>,
    /// All cache writes, across recorded lifetimes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<u64>,
    /// Five-minute cache writes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_5m: Option<u64>,
    /// One-hour cache writes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_1h: Option<u64>,
    /// Cache writes whose lifetime is unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_unspecified: Option<u64>,
    /// Output tokens, including reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    /// Reasoning tokens, a subset of output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
    /// Provider-specific additive tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_only: Option<u64>,
    /// Sum of every reported additive category; reasoning is not added twice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl TokenCounts {
    pub(crate) fn from_measures(measures: TokenMeasures) -> Result<Self, QueryError> {
        Ok(Self {
            uncached_input: measures.uncached_input,
            cache_read: measures.cache_read,
            cache_write: measures.cache_write()?,
            cache_write_5m: measures.cache_write_5m,
            cache_write_1h: measures.cache_write_1h,
            cache_write_unspecified: measures.cache_write_unspecified,
            output: measures.output,
            reasoning: measures.reasoning,
            provider_only: measures.provider_only,
            total: measures.total()?,
        })
    }
}

/// Non-counted usage shown beside, never added to, the main totals.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SideTotals {
    /// Requests in this side measure.
    pub requests: u64,
    /// Their tokens.
    pub tokens: TokenCounts,
}

/// The counted totals and the non-additive unresolved and possible measures.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AggregateTotals {
    /// Counted requests by ownership.
    pub requests: RequestCounts,
    /// Counted tokens.
    pub tokens: TokenCounts,
    /// Unresolved candidate-set members.
    pub unresolved: SideTotals,
    /// Ambiguous requests partly inside an explicit selection.
    pub possible: SideTotals,
}

/// Coverage counters for a report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CoverageSummary {
    /// Copy observations excluded from totals.
    pub copies_excluded: u64,
    /// Native provider limit observations retained.
    pub limit_observations: u64,
    /// Counted requests with no usage-bearing revision.
    pub requests_without_usage: u64,
    /// Whether no known usage is missing or unresolved.
    pub complete: bool,
}

/// One stable diagnostic code and occurrence count.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticSummary {
    /// Stable diagnostic token.
    pub code: String,
    /// Number of source-record or physical-location occurrences.
    pub count: u64,
    /// Deterministic detail from reconciliation.
    pub detail: String,
}

/// One grouped report row.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GroupRow {
    /// Group dimension.
    pub group: String,
    /// Group value, using `unknown` rather than an implicit null bucket.
    pub value: String,
    /// Counted requests by ownership.
    pub requests: RequestCounts,
    /// Counted tokens.
    pub tokens: TokenCounts,
}

/// Exact input-size percentiles over counted usage-bearing requests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SizeSummary {
    /// Number of requests with a reported inclusive input size.
    pub count: u64,
    /// Median inclusive input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50: Option<u64>,
    /// 90th percentile inclusive input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p90: Option<u64>,
    /// 99th percentile inclusive input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p99: Option<u64>,
    /// Largest inclusive input size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
}

/// Complete `report` command result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReportDocument {
    /// Rendering schema version.
    pub schema_version: u8,
    /// Normalized query.
    pub query: QueryMetadata,
    /// Coverage counters.
    pub coverage: CoverageSummary,
    /// Diagnostics in stable source order.
    pub diagnostics: Vec<DiagnosticSummary>,
    /// Counted and non-additive totals.
    pub totals: AggregateTotals,
    /// Requested dimension breakdowns, sorted by dimension then value.
    pub breakdowns: BTreeMap<String, Vec<GroupRow>>,
    /// Exact request input-size summary.
    pub sizes: SizeSummary,
}

/// One calendar row from `daily`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DailyRow {
    /// Calendar date in the report timezone; `None` when the request has no timestamp.
    pub date: Option<String>,
    /// Counted requests by ownership.
    pub requests: RequestCounts,
    /// Counted tokens.
    pub tokens: TokenCounts,
}

/// One session row from `sessions`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionRow {
    /// Analytical thread ID; `None` is the ambiguous or unknown-owner group.
    pub thread: Option<String>,
    /// Agent token, or `unknown` for the null group.
    pub agent: String,
    /// Project name from the thread, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Counted requests by ownership.
    pub requests: RequestCounts,
    /// Counted tokens.
    pub tokens: TokenCounts,
}

/// Complete `daily` command result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DailyDocument {
    /// Rendering schema version.
    pub schema_version: u8,
    /// Normalized query.
    pub query: QueryMetadata,
    /// Calendar rows in ascending date order, with the unknown bucket last.
    pub rows: Vec<DailyRow>,
    /// Diagnostics relevant to the selected sources.
    pub diagnostics: Vec<DiagnosticSummary>,
}

/// Complete `sessions` command result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionsDocument {
    /// Rendering schema version.
    pub schema_version: u8,
    /// Normalized query.
    pub query: QueryMetadata,
    /// Session rows in analytical-ID order.
    pub rows: Vec<SessionRow>,
    /// Diagnostics relevant to the selected sources.
    pub diagnostics: Vec<DiagnosticSummary>,
}

/// Query construction or aggregation failed.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    /// A timezone name was not available in the configured database.
    #[error("invalid timezone {name:?}: {source}")]
    InvalidTimeZone {
        /// Requested name.
        name: String,
        /// Timezone database error.
        #[source]
        source: jiff::Error,
    },
    /// A request or coverage counter overflowed.
    #[error("counter overflow in {0}")]
    CountOverflow(&'static str),
    /// Token aggregation overflowed.
    #[error(transparent)]
    Tokens(#[from] TokenOverflow),
}
