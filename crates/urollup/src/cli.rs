//! Argument parsing, stream handling and exit translation for the `urollup` executable.
//!
//! [`run_with_context`] takes stdout and stderr as injected writers plus explicit
//! terminal capabilities, so stream and exit behavior is unit-testable without spawning
//! a process. stdout carries only requested data (help and version text count as
//! requested); every diagnostic goes to stderr.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::io::{self, BufRead, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::builder::styling::{AnsiColor, Style as AnsiStyle, Styles};
use clap::{Args, ColorChoice, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use urollup_core::adapters::discovery::DiscoveryEnvironment;
use urollup_core::adapters::{AdapterError, Ingested, claude_project, codex_rollout, cursor_state};
use urollup_core::ledger::capacity::{
    ObservationCapacity, RamBudget, physical_memory_bytes, resolve_capacity,
};
use urollup_core::ledger::reconcile::ReconcileError;
use urollup_core::query::{
    GroupBy, QueryMetadata, QuerySource, ResolvedTimeZone, daily, report, sessions,
};
use urollup_core::selection::{
    Agent, CurrentEnvironment, Scope, SelectionError, SelectionQuery, SessionIndex,
    cursor_composer_id_from_selector, derive_agent_thread_id,
};
use urollup_core::sources::parallel;
use urollup_core::sources::reader::{self, ReadOptions};
use urollup_core::sources::roots::{self, DiscoveredSource, Discovery};

const STYLE_HEADING: AnsiStyle = AnsiColor::Cyan.on_default().bold();
const STYLE_ERROR: AnsiStyle = AnsiColor::Red.on_default().bold();
const MAX_CATALOG_HEADER_BYTES: u64 = ReadOptions::DEFAULT_MAX_RECORD_BYTES as u64;
/// The environment variable that sets how many threads decode sources.
const JOBS_VARIABLE: &str = "UROLLUP_JOBS";
/// The most workers `UROLLUP_JOBS` may request, so a mistyped value cannot exhaust the
/// process's threads.
const MAX_JOBS: usize = 256;
/// The environment variable that prints privacy-safe run statistics to stderr.
pub(crate) const STATS_VARIABLE: &str = "UROLLUP_STATS";
/// The environment variable that sets the ingest RAM budget when `--max-ram` is omitted.
const MAX_RAM_VARIABLE: &str = "UROLLUP_MAX_RAM";
const CLI_STYLES: Styles = Styles::styled()
    .header(STYLE_HEADING)
    .usage(STYLE_HEADING)
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Cyan.on_default())
    .error(STYLE_ERROR)
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Yellow.on_default());

/// Whether the process streams are attached to interactive terminals.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TerminalContext {
    pub(crate) stdout_is_terminal: bool,
    pub(crate) stderr_is_terminal: bool,
}

/// Color-related environment state, separated from policy so tests do not mutate the
/// process environment.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ColorEnvironment {
    no_color: bool,
    force_color: bool,
}

impl ColorEnvironment {
    pub(crate) fn from_process() -> Self {
        Self {
            no_color: std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
            force_color: std::env::var_os("FORCE_COLOR")
                .is_some_and(|value| !value.is_empty() && value != "0"),
        }
    }
}

/// The process exit classes milestone 0.1 can produce.
///
/// The full contract in design §6.5 also defines 3 (unmet coverage), 4 (exceeded
/// threshold) and 130 (interrupted); those variants are added with the features that
/// produce them, so no variant exists that nothing returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// Complete success.
    Success,
    /// Runtime failure, such as an I/O or write failure.
    Runtime,
    /// Invalid invocation or request.
    Usage,
}

impl Exit {
    /// The numeric process exit status for this class.
    pub fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Runtime => 1,
            Self::Usage => 2,
        }
    }
}

#[derive(Debug, Parser)]
// An explicit `bin_name` keeps help and usage text identical on every platform: clap would
// otherwise take it from argv[0], which is `urollup.exe` when Windows runs a full path.
#[command(name = "urollup", bin_name = "urollup", version, about, arg_required_else_help = true)]
struct Cli {
    /// Colorize human output: auto, always or never
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, global = true)]
    color: ColorWhen,

    /// Disable the interactive progress indicator
    #[arg(long, global = true)]
    no_progress: bool,

    #[command(subcommand)]
    command: Command,
}

/// When terminal styling should be enabled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum ColorWhen {
    /// Style human output only when its destination is a terminal.
    #[default]
    Auto,
    /// Style human output even when its destination is redirected.
    Always,
    /// Never style output.
    Never,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Session report: totals, coverage, request sizes, separate breakdowns and diagnostics
    Report(ReportArgs),
    /// Calendar rollup by day
    Daily(SelectionArgs),
    /// One row per session
    Sessions(SelectionArgs),
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Report(_) => "report",
            Self::Daily(_) => "daily",
            Self::Sessions(_) => "sessions",
        }
    }

    fn args(&self) -> &SelectionArgs {
        match self {
            Self::Report(args) => &args.selection,
            Self::Daily(args) | Self::Sessions(args) => args,
        }
    }

    const fn defaults_to_all(&self) -> bool {
        matches!(self, Self::Daily(_) | Self::Sessions(_))
    }
}

/// Session selection shared by every milestone 0.1 report command.
#[derive(Clone, Debug, Default, Args)]
struct SelectionArgs {
    /// Select the session running this command from an exact agent environment signal
    #[arg(long)]
    current: bool,

    /// Select a native ID, analytical thr- ID, or transcript path; repeatable
    #[arg(long = "session", value_name = "SELECTOR")]
    sessions: Vec<OsString>,

    /// Select every discovered session
    #[arg(long)]
    all: bool,

    /// Include only selected threads or also their spawned subagent descendants
    #[arg(long, value_enum, value_name = "SCOPE")]
    scope: Option<ScopeArg>,

    /// Output as a terminal table or JSON document
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,

    /// IANA timezone for calendar grouping; defaults to the system timezone
    #[arg(long, value_name = "ZONE")]
    timezone: Option<String>,

    /// Restrict selection to one or more agents; repeatable and comma-delimited
    #[arg(long, value_enum, value_delimiter = ',', value_name = "AGENT")]
    agent: Vec<AgentArg>,

    /// Add a source root or JSONL artifact; repeatable
    #[arg(long = "source", value_name = "PATH")]
    sources: Vec<PathBuf>,

    /// Read only paths named by --source
    #[arg(long)]
    no_default_sources: bool,

    /// Ingest budget as a byte size (512M, 8G, 8GiB) or a percent of physical RAM (25%). Default: 25% of RAM, or 2 GiB if RAM cannot be read. `UROLLUP_MAX_RAM` sets the same value when this flag is omitted
    #[arg(long, value_name = "SIZE")]
    max_ram: Option<String>,

    /// Exact per-agent observation ceiling. When set with --max-ram, the stricter (smaller) ceiling wins
    #[arg(long, value_name = "N")]
    max_rows: Option<u64>,
}

/// `report` arguments: the shared selection plus the breakdowns only `report` prints.
///
/// `daily` and `sessions` do not take `--group-by`, so clap rejects it there as a usage
/// error instead of accepting a flag the rollups would ignore. Joint calendar and
/// dimension grouping is planned separately (uro-qvp1).
#[derive(Clone, Debug, Default, Args)]
struct ReportArgs {
    #[command(flatten)]
    selection: SelectionArgs,

    /// One separate breakdown per dimension; dimensions are never combined. Repeatable and comma-delimited. Default: project, account, model and effort
    // Help lists options by display order, then by long name. Clap numbers the flattened
    // selection from 0, so 7 is `--source`'s slot: `--group-by` follows `--timezone` and
    // `--agent` and precedes `--source`. The CLI-surface golden pins the order.
    #[arg(long, value_enum, value_delimiter = ',', value_name = "DIMENSION", display_order = 7)]
    group_by: Vec<GroupByArg>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ScopeArg {
    #[value(name = "self")]
    SelfOnly,
    Descendants,
}

impl From<ScopeArg> for Scope {
    fn from(value: ScopeArg) -> Self {
        match value {
            ScopeArg::SelfOnly => Self::SelfOnly,
            ScopeArg::Descendants => Self::Descendants,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    #[default]
    Table,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum GroupByArg {
    Project,
    Account,
    Model,
    Effort,
    Agent,
    Provider,
    Purpose,
}

impl From<GroupByArg> for GroupBy {
    fn from(value: GroupByArg) -> Self {
        match value {
            GroupByArg::Project => Self::Project,
            GroupByArg::Account => Self::Account,
            GroupByArg::Model => Self::Model,
            GroupByArg::Effort => Self::Effort,
            GroupByArg::Agent => Self::Agent,
            GroupByArg::Provider => Self::Provider,
            GroupByArg::Purpose => Self::Purpose,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum AgentArg {
    Claude,
    Codex,
    Cursor,
    Pi,
}

impl From<AgentArg> for Agent {
    fn from(value: AgentArg) -> Self {
        match value {
            AgentArg::Claude => Self::Claude,
            AgentArg::Codex => Self::Codex,
            AgentArg::Cursor => Self::Cursor,
            AgentArg::Pi => Self::Pi,
        }
    }
}

/// Parse `args` (including the program name), perform the command and return its exit
/// class.
///
/// Output written to `stdout` is flushed before a success is reported, so a failed write
/// can never be reported as success. A consumer that closes stdout after the output was
/// produced is still success (design §6.5).
#[cfg(test)]
fn run<I, T>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> Exit
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    run_with_context(
        args,
        stdout,
        stderr,
        TerminalContext::default(),
        ColorEnvironment::default(),
        None,
    )
}

/// Run with explicit terminal and environment capabilities.
///
/// Keeping these inputs separate from the writers makes the TTY branches deterministic
/// under unit tests while `main` still probes the real destination streams.
pub(crate) fn run_with_context<I, T>(
    args: I,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    terminals: TerminalContext,
    color_environment: ColorEnvironment,
    stats: Option<&OsStr>,
) -> Exit
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let requested_color = requested_color(&args);
    let machine_requested = machine_format_requested(&args);
    let command = Cli::command().styles(CLI_STYLES).color(ColorChoice::Always);
    let matches = match command.try_get_matches_from(&args) {
        Ok(matches) => matches,
        Err(error) => {
            return report_parse_outcome(
                &error,
                stdout,
                stderr,
                terminals,
                ColorContext {
                    when: requested_color,
                    machine: machine_requested,
                    environment: color_environment,
                },
            );
        }
    };
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return Exit::Usage;
        }
    };
    let machine = cli.command.args().format == OutputFormat::Json;
    let stdout_color = ColorContext { when: cli.color, machine, environment: color_environment }
        .enabled(terminals.stdout_is_terminal);
    let stderr_color = ColorContext { when: cli.color, machine, environment: color_environment }
        .enabled(terminals.stderr_is_terminal);
    let show_stats = match stats_requested(stats) {
        Ok(show_stats) => show_stats,
        Err(failure) => return report_failure(stderr, &failure, stderr_color),
    };
    let progress = !cli.no_progress && !machine && terminals.stderr_is_terminal;
    if progress {
        write_progress(stderr, "Reading usage logs…");
    }
    let mut stats = Stats::default();
    let started = Instant::now();
    let result = execute(&cli.command, stdout_color, &mut stats);
    let total = started.elapsed();
    if progress {
        clear_progress(stderr);
    }
    if show_stats {
        // Statistics are diagnostics: a closed stderr must not change the outcome.
        let _ = stderr.write_all(stats.render(total).as_bytes()).and_then(|()| stderr.flush());
    }
    match result {
        Ok(output) => finish_stdout(
            stdout.write_all(output.as_bytes()).and_then(|()| stdout.flush()),
            stderr,
            stderr_color,
        ),
        Err(failure) => report_failure(stderr, &failure, stderr_color),
    }
}

/// Write a failure's diagnostic to stderr and return its exit class.
fn report_failure(stderr: &mut dyn Write, failure: &Failure, color: bool) -> Exit {
    let label = paint("error:", STYLE_ERROR, color);
    let _ = writeln!(stderr, "{label} {}", failure.message);
    failure.exit
}

struct Corpus {
    claude: Ingested,
    codex: Ingested,
    cursor: Ingested,
    index: SessionIndex,
}

impl Corpus {
    fn discover(
        args: &SelectionArgs,
        query: &SelectionQuery,
        stats: &mut Stats,
    ) -> Result<Self, Failure> {
        let started = Instant::now();
        let workers = decoding_workers(std::env::var_os(JOBS_VARIABLE).as_deref())?;
        let capacity = ingest_capacity(
            args.max_ram.as_deref(),
            args.max_rows,
            std::env::var_os(MAX_RAM_VARIABLE).as_deref(),
            physical_memory_bytes,
        )?;
        stats.workers = Some(workers);
        let environment = DiscoveryEnvironment::from_process();
        let (mut claude_roots, claude_missing_is_error) = if args.no_default_sources {
            (Vec::new(), false)
        } else {
            let selected = environment.claude_project_roots();
            let missing_is_error = selected.missing_is_error();
            (selected.roots, missing_is_error)
        };
        let (mut codex_roots, codex_missing_is_error) = if args.no_default_sources {
            (Vec::new(), false)
        } else {
            let selected = environment.codex_homes();
            let missing_is_error = selected.missing_is_error();
            (selected.roots, missing_is_error)
        };
        let (mut cursor_roots, cursor_missing_is_error) = if args.no_default_sources {
            (Vec::new(), false)
        } else {
            let selected = environment.cursor_roots();
            let missing_is_error = selected.missing_is_error();
            (selected.roots, missing_is_error)
        };
        for source in &args.sources {
            for explicit in classify_explicit_source(source)? {
                match explicit {
                    ExplicitDialect::Claude(root) => claude_roots.push(root),
                    ExplicitDialect::Codex(root) => codex_roots.push(root),
                    ExplicitDialect::Cursor(root) => cursor_roots.push(root),
                }
            }
        }
        let mut claude_discovery = roots::discover(&claude_roots);
        let codex_roots = codex_rollout::rollout_roots(&codex_roots);
        let mut codex_discovery = roots::discover(&codex_roots);
        narrow_discoveries(&mut claude_discovery, &mut codex_discovery, query)?;
        stats.phase("discovery", started);

        // Codex is the larger corpus. Index and drop its discovery tables before Claude
        // starts, so the peak is one ingest working set plus the other's compact ledger.
        let mut index = SessionIndex::default();
        let mut index_elapsed = Duration::ZERO;

        let started = Instant::now();
        let mut codex = codex_rollout::ingest_discovery_with_capacity(
            codex_discovery,
            codex_missing_is_error,
            workers,
            &capacity,
        )
        .map_err(|error| Failure::adapter(&error))?;
        stats.phase("codex_ingest", started);
        stats.agent("codex", &codex);
        let started = Instant::now();
        index.add(Agent::Codex, &codex).map_err(|error| Failure::selection(&error))?;
        codex.release_discovery();
        index_elapsed += started.elapsed();

        let started = Instant::now();
        let mut claude = claude_project::ingest_discovery_with_capacity(
            claude_discovery,
            claude_missing_is_error,
            workers,
            &capacity,
        )
        .map_err(|error| Failure::adapter(&error))?;
        stats.phase("claude_ingest", started);
        stats.agent("claude", &claude);
        let started = Instant::now();
        index.add(Agent::Claude, &claude).map_err(|error| Failure::selection(&error))?;
        claude.release_discovery();
        index_elapsed += started.elapsed();

        let started = Instant::now();
        let mut cursor = cursor_state::ingest_roots(
            &cursor_roots,
            cursor_missing_is_error,
            &capacity,
            cursor_composer_ids(query).as_deref(),
        )
        .map_err(|error| Failure::adapter(&error))?;
        stats.phase("cursor_ingest", started);
        stats.agent("cursor", &cursor);
        let started = Instant::now();
        index.add(Agent::Cursor, &cursor).map_err(|error| Failure::selection(&error))?;
        cursor.release_discovery();
        index_elapsed += started.elapsed();

        stats.record("session_index", index_elapsed);
        Ok(Self { claude, codex, cursor, index })
    }

    fn sources(&self) -> [QuerySource<'_>; 3] {
        [
            QuerySource { agent: Agent::Claude, ingested: &self.claude },
            QuerySource { agent: Agent::Codex, ingested: &self.codex },
            QuerySource { agent: Agent::Cursor, ingested: &self.cursor },
        ]
    }

    fn has_cursor_sessions(&self) -> bool {
        self.index.sessions().any(|(_, session)| session.agent == Agent::Cursor)
    }
}

/// Native Cursor composer UUIDs from `--current` / `--session`, used to skip unread
/// composers and their bubbles. Analytical `thr-` IDs and path selectors need the
/// full store.
fn cursor_composer_ids(query: &SelectionQuery) -> Option<Vec<String>> {
    let mut ids = Vec::new();
    if let Some(current) = &query.current {
        if current.agent == Agent::Cursor {
            ids.push(cursor_composer_id_from_selector(&current.selector)?);
        }
    }
    for session in &query.sessions {
        match cursor_composer_id_from_selector(session.as_os_str()) {
            Some(id) => ids.push(id),
            None if session.to_str().is_some_and(|selector| selector.starts_with("thr-")) => {
                return None;
            }
            None => return None,
        }
    }
    if ids.is_empty() { None } else { Some(ids) }
}

/// The number of threads that decode sources: `UROLLUP_JOBS` when it is set, otherwise
/// the core's default bound.
///
/// An empty value counts as unset, as it does for `NO_COLOR` and `FORCE_COLOR`. Anything
/// but a whole number from 1 to [`MAX_JOBS`] is a usage error.
fn decoding_workers(jobs: Option<&OsStr>) -> Result<NonZeroUsize, Failure> {
    let Some(jobs) = jobs.filter(|jobs| !jobs.is_empty()) else {
        return Ok(parallel::default_workers());
    };
    jobs.to_str()
        .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|text| text.parse::<NonZeroUsize>().ok())
        .filter(|workers| workers.get() <= MAX_JOBS)
        .ok_or_else(|| {
            Failure::usage(format!(
                "{JOBS_VARIABLE} must be a whole number of workers from 1 to {MAX_JOBS}, not {:?}",
                jobs.to_string_lossy()
            ))
        })
}

/// The per-agent observation ceiling: `--max-ram` (or `UROLLUP_MAX_RAM`) and `--max-rows`.
///
/// Empty `UROLLUP_MAX_RAM` counts as unset, as it does for `UROLLUP_JOBS`. `--max-ram`
/// replaces the environment value. `--max-rows` alone is an exact override of the 25% RAM
/// default; when both controls are set, the stricter (smaller) ceiling wins.
fn ingest_capacity(
    max_ram: Option<&str>,
    max_rows: Option<u64>,
    env_max_ram: Option<&OsStr>,
    physical_ram: impl FnOnce() -> Option<u64>,
) -> Result<ObservationCapacity, Failure> {
    let ram =
        if let Some(text) = max_ram {
            Some(parse_ram_budget(text)?)
        } else if let Some(value) = env_max_ram.filter(|value| !value.is_empty()) {
            let text = value.to_str().ok_or_else(|| {
                Failure::usage(format!(
                    "{MAX_RAM_VARIABLE} must be a UTF-8 byte size such as 512M or 25%, not {:?}",
                    value.to_string_lossy()
                ))
            })?;
            Some(RamBudget::parse(text).map_err(|error| {
                Failure::usage(format!("{MAX_RAM_VARIABLE} is invalid: {error}"))
            })?)
        } else {
            None
        };
    let physical_ram = match (ram, max_rows) {
        (Some(RamBudget::Percent(_)), _) | (None, None) => physical_ram(),
        (Some(RamBudget::Bytes(_)), _) | (None, Some(_)) => None,
    };
    resolve_capacity(ram, max_rows, physical_ram).map_err(|error| Failure::usage(error.to_string()))
}

fn parse_ram_budget(text: &str) -> Result<RamBudget, Failure> {
    RamBudget::parse(text).map_err(|error| Failure::usage(error.to_string()))
}

/// Whether `UROLLUP_STATS` asks for run statistics.
///
/// Only `1` enables them. An empty value counts as unset, as it does for `UROLLUP_JOBS`,
/// and `0` disables them; anything else is a usage error rather than a silent no.
fn stats_requested(value: Option<&OsStr>) -> Result<bool, Failure> {
    let Some(value) = value else { return Ok(false) };
    match value.as_encoded_bytes() {
        b"" | b"0" => Ok(false),
        b"1" => Ok(true),
        _ => Err(Failure::usage(format!(
            "{STATS_VARIABLE} must be 1 to print run statistics, or 0 or empty to disable them, not {:?}",
            value.to_string_lossy()
        ))),
    }
}

/// Privacy-safe measurements of one command, which `UROLLUP_STATS=1` prints to stderr.
///
/// Only phase names, wall times, the worker count and row counts are kept, never a path,
/// ID or model name, so the lines can be shared from a private corpus.
#[derive(Debug, Default)]
struct Stats {
    workers: Option<NonZeroUsize>,
    phases: Vec<(&'static str, Duration)>,
    agents: Vec<AgentStats>,
}

/// Row counts from one agent's ingestion.
#[derive(Debug, Eq, PartialEq)]
struct AgentStats {
    agent: &'static str,
    sources: usize,
    observations: u64,
    requests: usize,
    limit_observations: usize,
    diagnostics: usize,
}

impl Stats {
    /// Records a phase that began at `started` and ends now.
    fn phase(&mut self, name: &'static str, started: Instant) {
        self.record(name, started.elapsed());
    }

    /// Records a phase whose duration was accumulated across non-contiguous steps.
    fn record(&mut self, name: &'static str, elapsed: Duration) {
        self.phases.push((name, elapsed));
    }

    fn agent(&mut self, agent: &'static str, ingested: &Ingested) {
        self.agents.push(AgentStats {
            agent,
            sources: ingested.manifest.entries.len(),
            observations: ingested.ledger.coverage.observations,
            requests: ingested.ledger.requests.len(),
            limit_observations: ingested.limit_observations.len(),
            diagnostics: ingested.ledger.diagnostics.len(),
        });
    }

    /// One `stats:` line per measurement in `key=value` form, ending with the command's
    /// total wall time. Phases that a failure prevented are absent.
    fn render(&self, total: Duration) -> String {
        let mut lines = String::new();
        if let Some(workers) = self.workers {
            let _ = writeln!(lines, "stats: workers={workers}");
        }
        for (name, elapsed) in &self.phases {
            let _ = writeln!(lines, "stats: phase={name} seconds={:.3}", elapsed.as_secs_f64());
        }
        for agent in &self.agents {
            let _ = writeln!(
                lines,
                "stats: agent={} sources={} observations={} requests={} limit_observations={} diagnostics={}",
                agent.agent,
                agent.sources,
                agent.observations,
                agent.requests,
                agent.limit_observations,
                agent.diagnostics
            );
        }
        let _ = writeln!(lines, "stats: total seconds={:.3}", total.as_secs_f64());
        lines
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SourceFamily {
    Claude(PathBuf),
    Codex(String),
}

#[derive(Clone, Debug)]
struct CatalogSource {
    agent: Agent,
    family: SourceFamily,
    native_selector: OsString,
    analytical_selector: String,
    paths: Vec<PathBuf>,
    /// Whether no file of the source had a readable header, so its family is unknown.
    unread: bool,
}

impl CatalogSource {
    fn matches(&self, selector: &OsStr, agent: Option<Agent>) -> bool {
        if agent.is_some_and(|expected| self.agent != expected) {
            return false;
        }
        if selector == self.native_selector
            || selector.to_str() == Some(self.analytical_selector.as_str())
        {
            return true;
        }
        let selected_path = Path::new(selector);
        let canonical = std::fs::canonicalize(selected_path).ok();
        self.paths.iter().any(|path| {
            path == selected_path
                || canonical.as_ref().is_some_and(|selected| {
                    std::fs::canonicalize(path).is_ok_and(|path| path == *selected)
                })
        })
    }
}

/// Restrict discovery to session families named by exact native IDs or transcript paths.
///
/// Ordinary analytical IDs are derived from catalog native IDs. Analytical IDs for
/// inline Claude sidechains cannot be recovered without decoding the transcript, so an
/// unmatched `thr-` selector deliberately falls back to full ingestion. Family-level
/// retention keeps the records needed to reconcile copies and determine the final
/// descendant selection without reading unrelated runs.
fn narrow_discoveries(
    claude: &mut Discovery,
    codex: &mut Discovery,
    query: &SelectionQuery,
) -> Result<bool, Failure> {
    let mut selectors: Vec<(Option<Agent>, &OsStr)> = Vec::new();
    if let Some(current) = &query.current {
        selectors.push((Some(current.agent), &current.selector));
    }
    selectors.extend(query.sessions.iter().map(|selector| (None, selector.as_os_str())));
    if selectors.is_empty() {
        return Ok(false);
    }

    let needs_claude =
        selectors.iter().any(|(agent, _)| agent.is_none_or(|agent| agent == Agent::Claude));
    let needs_codex =
        selectors.iter().any(|(agent, _)| agent.is_none_or(|agent| agent == Agent::Codex));
    let claude_catalog: Vec<_> = if needs_claude {
        claude.sources.iter().map(claude_catalog_source).collect::<Result<_, _>>()?
    } else {
        Vec::new()
    };
    let codex_catalog: Vec<_> =
        if needs_codex { codex_catalog_sources(codex)? } else { Vec::new() };
    let mut selected_families = BTreeSet::new();
    for (agent, selector) in selectors {
        let matching: Vec<_> = claude_catalog
            .iter()
            .chain(&codex_catalog)
            .filter(|source| source.matches(selector, agent))
            .map(|source| source.family.clone())
            .collect();
        if matching.is_empty()
            && selector.to_str().is_some_and(|selector| selector.starts_with("thr-"))
        {
            return Ok(false);
        }
        selected_families.extend(matching);
    }

    if needs_claude {
        retain_catalog_families(claude, claude_catalog, &selected_families);
    } else {
        claude.sources.clear();
    }
    if needs_codex {
        retain_catalog_families(codex, codex_catalog, &selected_families);
    } else {
        codex.sources.clear();
    }
    Ok(true)
}

fn retain_catalog_families(
    discovery: &mut Discovery,
    catalog: Vec<CatalogSource>,
    selected: &BTreeSet<SourceFamily>,
) {
    discovery.sources = std::mem::take(&mut discovery.sources)
        .into_iter()
        .zip(catalog)
        // A source whose header could not be read may belong to any family, so it is left
        // for ingest, which reads it or reports it as incomplete.
        .filter_map(|(source, catalog)| {
            (catalog.unread || selected.contains(&catalog.family)).then_some(source)
        })
        .collect();
}

fn claude_catalog_source(source: &DiscoveredSource) -> Result<CatalogSource, Failure> {
    let locator = PathBuf::from(&source.locator);
    let components: Vec<_> = locator.components().collect();
    let subagents =
        components.iter().position(|component| component.as_os_str() == OsStr::new("subagents"));
    let (family_relative, native_selector, identity_native) = match subagents {
        Some(index) if index > 0 => {
            let family = components[..index].iter().fold(PathBuf::new(), |mut path, component| {
                path.push(component.as_os_str());
                path
            });
            let native = locator.file_stem().map_or_else(OsString::new, |stem| {
                let stem = stem.to_string_lossy();
                OsString::from(stem.strip_prefix("agent-").unwrap_or(&stem))
            });
            let session = components[index.saturating_sub(1)].as_os_str().to_string_lossy();
            let identity_native = format!("{session}/{}", native.to_string_lossy());
            (family, native, identity_native)
        }
        _ => {
            let native = locator.file_stem().map_or_else(OsString::new, OsString::from);
            (locator.with_extension(""), native.clone(), native.to_string_lossy().into_owned())
        }
    };
    let analytical_selector = derive_agent_thread_id(Agent::Claude, &identity_native)
        .map_err(|error| Failure::runtime(error.to_string()))?
        .to_string();
    Ok(CatalogSource {
        agent: Agent::Claude,
        family: SourceFamily::Claude(source.root.join(family_relative)),
        native_selector,
        analytical_selector,
        paths: source_paths(source),
        unread: false,
    })
}

fn codex_catalog_sources(discovery: &Discovery) -> Result<Vec<CatalogSource>, Failure> {
    let mut headers = Vec::with_capacity(discovery.sources.len());
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for source in &discovery.sources {
        let thread = codex_thread_from_locator(&source.locator);
        adjacency.entry(thread.clone()).or_default();
        let links = read_codex_session_links(source);
        for linked in links.iter().flatten() {
            adjacency.entry(thread.clone()).or_default().insert(linked.clone());
            adjacency.entry(linked.clone()).or_default().insert(thread.clone());
        }
        headers.push((thread, source, links.is_none()));
    }

    let mut component_by_thread = BTreeMap::new();
    let mut remaining: BTreeSet<_> = adjacency.keys().cloned().collect();
    while let Some(start) = remaining.first().cloned() {
        let mut members = BTreeSet::new();
        let mut queue = VecDeque::from([start]);
        while let Some(thread) = queue.pop_front() {
            if !members.insert(thread.clone()) {
                continue;
            }
            remaining.remove(&thread);
            queue.extend(adjacency.get(&thread).into_iter().flatten().cloned());
        }
        let family = members.first().expect("a component has its starting thread").clone();
        component_by_thread.extend(members.into_iter().map(|thread| (thread, family.clone())));
    }

    headers
        .into_iter()
        .map(|(thread, source, unread)| {
            let analytical_selector = derive_agent_thread_id(Agent::Codex, &thread)
                .map_err(|error| Failure::runtime(error.to_string()))?
                .to_string();
            Ok(CatalogSource {
                agent: Agent::Codex,
                family: SourceFamily::Codex(
                    component_by_thread.get(&thread).cloned().unwrap_or_else(|| thread.clone()),
                ),
                native_selector: OsString::from(thread),
                analytical_selector,
                paths: source_paths(source),
                unread,
            })
        })
        .collect()
}

fn source_paths(source: &DiscoveredSource) -> Vec<PathBuf> {
    source.files.files().map(|(path, _)| path.to_owned()).collect()
}

fn codex_thread_from_locator(locator: &str) -> String {
    let name = Path::new(locator)
        .file_name()
        .map_or_else(|| locator.to_owned(), |name| name.to_string_lossy().into_owned());
    let stem = name.strip_suffix(".jsonl").unwrap_or(&name);
    if let Some((base, _rollout)) = stem.rsplit_once('_') {
        return base.get(base.len().saturating_sub(36)..).unwrap_or(base).to_owned();
    }
    stem.get(stem.len().saturating_sub(36)..).unwrap_or(stem).to_owned()
}

/// The threads a rollout's `session_meta` header links: its session, parent and fork
/// origin. The header is the first record of the first file of the source that has one,
/// read under the reader's rules, so a primary compressed since discovery is read from its
/// new file. `None` when no file has a complete first record, as while a compressor is
/// still writing it or when it is damaged: ingest then reads the source or reports it.
fn read_codex_session_links(source: &DiscoveredSource) -> Option<BTreeSet<String>> {
    reader::peek(&source.files, |reader| {
        let mut line = Vec::new();
        reader
            .take(MAX_CATALOG_HEADER_BYTES.saturating_add(1))
            .read_until(b'\n', &mut line)
            .ok()?;
        // Only a terminated line within the record limit is a complete first record.
        if line.last() != Some(&b'\n') {
            return None;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) else {
            return Some(BTreeSet::new());
        };
        if value.get("type").and_then(serde_json::Value::as_str) != Some("session_meta") {
            return Some(BTreeSet::new());
        }
        Some(
            [
                "/payload/session_id",
                "/payload/parent_thread_id",
                "/payload/forked_from_id",
                "/payload/source/subagent/thread_spawn/parent_thread_id",
            ]
            .into_iter()
            .filter_map(|pointer| value.pointer(pointer).and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            .collect(),
        )
    })
}

#[derive(Debug)]
enum ExplicitDialect {
    Claude(PathBuf),
    Codex(PathBuf),
    Cursor(PathBuf),
}

fn classify_explicit_source(source: &Path) -> Result<Vec<ExplicitDialect>, Failure> {
    if cursor_state::is_cursor_source(source) {
        return Ok(vec![ExplicitDialect::Cursor(source.to_owned())]);
    }
    let mut standard_roots = Vec::new();
    if source.join("projects").is_dir() {
        standard_roots.push(ExplicitDialect::Claude(source.join("projects")));
    }
    if source.join("sessions").is_dir() || source.join("archived_sessions").is_dir() {
        standard_roots.push(ExplicitDialect::Codex(source.to_owned()));
    }
    if !standard_roots.is_empty() {
        return Ok(standard_roots);
    }

    let discovery = roots::discover(&[source.to_owned()]);
    if let Some(missing) = discovery.missing_roots.first() {
        return Err(Failure::runtime(format!("source root does not exist: {}", missing.display())));
    }
    if let Some(unreadable) = discovery.unreadable.first() {
        return Err(Failure::runtime(format!(
            "cannot inspect source path {}: {:?}",
            unreadable.path.display(),
            unreadable.kind
        )));
    }
    let mut dialect = None;
    for discovered in &discovery.sources {
        // A source that cannot be read, or that holds only records no dialect claims, such
        // as a lone title record, says nothing about the root; the other files decide it,
        // and ingest reports an unreadable one.
        let Some(candidate) = reader::peek(&discovered.files, |reader| {
            classify_records(reader, MAX_CATALOG_HEADER_BYTES)
        }) else {
            continue;
        };
        if dialect.replace(candidate).is_some_and(|previous| previous != candidate) {
            return Err(Failure::runtime(format!(
                "source {} contains multiple agent dialects; pass each agent root separately",
                source.display()
            )));
        }
    }
    match dialect {
        Some(Agent::Claude) => Ok(vec![ExplicitDialect::Claude(source.to_owned())]),
        Some(Agent::Codex) => Ok(vec![ExplicitDialect::Codex(source.to_owned())]),
        Some(Agent::Cursor) => Err(Failure::runtime(format!(
            "source {} is a Cursor JSONL transcript, which is not a usage owner; pass state.vscdb or a cursor-state fixture",
            source.display()
        ))),
        Some(Agent::Pi) => Err(Failure::usage("Pi source input is not supported yet")),
        None => Err(Failure::runtime(format!(
            "source {} contains no readable Claude Code, Codex or Cursor records",
            source.display()
        ))),
    }
}

/// Classifies decoded records by the first recognized record type. `None` when none of
/// the first 100 records belongs to a supported dialect, or when the records cannot be
/// read, a record within `max_line_bytes` included, so such a file decides nothing.
fn classify_records(reader: &mut dyn BufRead, max_line_bytes: u64) -> Option<Agent> {
    let mut line = Vec::new();
    for _ in 0..100 {
        line.clear();
        let read =
            reader.take(max_line_bytes.saturating_add(1)).read_until(b'\n', &mut line).ok()?;
        if read == 0 {
            break;
        }
        if u64::try_from(line.len()).unwrap_or(u64::MAX) > max_line_bytes {
            return None;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) else { continue };
        let kind = value.get("type").and_then(serde_json::Value::as_str);
        if matches!(
            kind,
            Some(
                "session_meta"
                    | "turn_context"
                    | "token_usage_record"
                    | "compacted"
                    | "event_msg"
                    | "response_item"
            )
        ) {
            return Some(Agent::Codex);
        }
        if matches!(
            kind,
            Some(
                "assistant"
                    | "user"
                    | "progress"
                    | "system"
                    | "summary"
                    | "queue-operation"
                    | "file-history-snapshot"
                    | "attachment"
                    | "bridge-session"
                    | "custom-title"
            )
        ) {
            return Some(Agent::Claude);
        }
        if kind == Some("turn_ended")
            || (kind.is_none()
                && matches!(
                    value.get("role").and_then(serde_json::Value::as_str),
                    Some("user" | "assistant")
                )
                && value.get("message").is_some())
        {
            return Some(Agent::Cursor);
        }
    }
    None
}

#[derive(Debug)]
struct Failure {
    exit: Exit,
    message: String,
}

impl Failure {
    fn usage(message: impl Into<String>) -> Self {
        Self { exit: Exit::Usage, message: message.into() }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self { exit: Exit::Runtime, message: message.into() }
    }

    fn adapter(error: &AdapterError) -> Self {
        if matches!(error, AdapterError::Reconcile(ReconcileError::CapacityExceeded { .. })) {
            return Self::runtime(format!(
                "{error}; pass narrower --source roots with --no-default-sources"
            ));
        }
        Self::runtime(error.to_string())
    }

    fn selection(error: &SelectionError) -> Self {
        let exit = match error {
            SelectionError::UnknownSelector(_)
            | SelectionError::ConflictingThread(_)
            | SelectionError::UnsavedSession { .. } => Exit::Runtime,
            SelectionError::AmbiguousSelector { .. }
            | SelectionError::NoSelector
            | SelectionError::CurrentNotDetected
            | SelectionError::AmbiguousCurrent(_)
            | SelectionError::UnsupportedAgent(_)
            | SelectionError::HookJson { .. }
            | SelectionError::HookField(_)
            | SelectionError::HookMismatch => Exit::Usage,
        };
        Self { exit, message: error.to_string() }
    }
}

fn execute(command: &Command, color: bool, stats: &mut Stats) -> Result<String, Failure> {
    let args = command.args();
    let timezone = ResolvedTimeZone::resolve(args.timezone.as_deref())
        .map_err(|error| Failure::usage(error.to_string()))?;
    let explicit_selection = args.current || !args.sessions.is_empty() || args.all;
    let wants_current = args.current
        || (!explicit_selection && args.sources.is_empty() && !command.defaults_to_all());
    let wants_all = args.all
        || (!explicit_selection && (command.defaults_to_all() || !args.sources.is_empty()));
    let agents: BTreeSet<Agent> = args.agent.iter().copied().map(Agent::from).collect();
    let current = wants_current
        .then(|| CurrentEnvironment::from_process().detect(&agents))
        .transpose()
        .map_err(|error| Failure::selection(&error))?;
    let scope = args.scope.map_or_else(
        || {
            if wants_current || !args.sessions.is_empty() {
                Scope::Descendants
            } else {
                Scope::SelfOnly
            }
        },
        Scope::from,
    );
    let mut query = SelectionQuery {
        current,
        sessions: args.sessions.clone(),
        all: wants_all,
        scope: Some(scope),
        agents,
    };
    let corpus = Corpus::discover(args, &query, stats)?;
    if query.current.as_ref().is_some_and(|current| current.agent == Agent::Cursor)
        && !corpus.has_cursor_sessions()
    {
        query.current = None;
        if wants_current && query.sessions.is_empty() && !wants_all {
            return Err(Failure::selection(&SelectionError::CurrentNotDetected));
        }
    }
    let selection_name = selection_name(&query);
    let started = Instant::now();
    let selected = corpus.index.select(&query).map_err(|error| Failure::selection(&error))?;
    let all = query.all
        && query.current.is_none()
        && query.sessions.is_empty()
        && query.agents.is_empty();
    let sources = corpus.sources();
    let metadata = QueryMetadata::new(command.name(), selection_name, scope, &timezone);

    let mut output = match command {
        Command::Report(report_args) => {
            let groups = report_args.group_by.iter().copied().map(GroupBy::from).collect();
            let document = report(&sources, &corpus.index, &selected, all, metadata, &groups)
                .map_err(|error| Failure::runtime(error.to_string()))?;
            match args.format {
                OutputFormat::Table => crate::render::report(&document, color),
                OutputFormat::Json => serde_json::to_string_pretty(&document)
                    .map_err(|error| Failure::runtime(error.to_string()))?,
            }
        }
        Command::Daily(_) => {
            let document = daily(&sources, &selected, all, metadata, &timezone)
                .map_err(|error| Failure::runtime(error.to_string()))?;
            match args.format {
                OutputFormat::Table => crate::render::daily(&document, color),
                OutputFormat::Json => serde_json::to_string_pretty(&document)
                    .map_err(|error| Failure::runtime(error.to_string()))?,
            }
        }
        Command::Sessions(_) => {
            let document = sessions(&sources, &corpus.index, &selected, all, metadata, &timezone)
                .map_err(|error| Failure::runtime(error.to_string()))?;
            match args.format {
                OutputFormat::Table => crate::render::sessions(&document, color),
                OutputFormat::Json => serde_json::to_string_pretty(&document)
                    .map_err(|error| Failure::runtime(error.to_string()))?,
            }
        }
    };
    if !output.ends_with('\n') {
        output.push('\n');
    }
    stats.phase("query_render", started);
    Ok(output)
}

fn selection_name(query: &SelectionQuery) -> String {
    let mut parts = Vec::new();
    if query.current.is_some() {
        parts.push("current");
    }
    if !query.sessions.is_empty() {
        parts.push("session");
    }
    if query.all {
        parts.push("all");
    }
    parts.join("+")
}

/// Render clap's help, version or usage-error outcome to the stream it belongs on.
fn report_parse_outcome(
    error: &clap::Error,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    terminals: TerminalContext,
    color: ColorContext,
) -> Exit {
    let rendered = error.render();
    if error.use_stderr() {
        let rendered = styled_text(&rendered, color.enabled(terminals.stderr_is_terminal));
        // A closed stderr must not change a status that is already decided.
        let _ = stderr.write_all(rendered.as_bytes()).and_then(|()| stderr.flush());
        return Exit::Usage;
    }
    let rendered = styled_text(&rendered, color.enabled(terminals.stdout_is_terminal));
    // `--help` and `--version` are requested data, so they go to stdout.
    finish_stdout(
        stdout.write_all(rendered.as_bytes()).and_then(|()| stdout.flush()),
        stderr,
        color.enabled(terminals.stderr_is_terminal),
    )
}

fn requested_color(args: &[OsString]) -> ColorWhen {
    let mut arguments = args.iter().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            break;
        }
        if argument == "--color" {
            return arguments.next().and_then(|value| color_value(value)).unwrap_or_default();
        }
        if let Some(value) = argument.to_str().and_then(|value| value.strip_prefix("--color=")) {
            return color_value(OsStr::new(value)).unwrap_or_default();
        }
    }
    ColorWhen::Auto
}

fn color_value(value: &OsStr) -> Option<ColorWhen> {
    match value.to_str()? {
        "auto" => Some(ColorWhen::Auto),
        "always" => Some(ColorWhen::Always),
        "never" => Some(ColorWhen::Never),
        _ => None,
    }
}

fn machine_format_requested(args: &[OsString]) -> bool {
    let mut arguments = args.iter().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            break;
        }
        if argument == "--format" {
            return arguments.next().is_some_and(|value| value == "json");
        }
        if argument.to_str().and_then(|value| value.strip_prefix("--format=")) == Some("json") {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ColorContext {
    when: ColorWhen,
    machine: bool,
    environment: ColorEnvironment,
}

impl ColorContext {
    fn enabled(self, destination_is_terminal: bool) -> bool {
        if self.machine || self.when == ColorWhen::Never || self.environment.no_color {
            return false;
        }
        match self.when {
            ColorWhen::Always => true,
            ColorWhen::Auto if self.environment.force_color => true,
            ColorWhen::Auto => destination_is_terminal,
            ColorWhen::Never => false,
        }
    }
}

fn styled_text(rendered: &clap::builder::StyledStr, color: bool) -> String {
    let rendered = if color { rendered.ansi().to_string() } else { rendered.to_string() };
    let mut output = String::with_capacity(rendered.len());
    for line in rendered.split_inclusive('\n') {
        let (content, newline) =
            line.strip_suffix('\n').map_or((line, ""), |content| (content, "\n"));
        output.push_str(content.trim_end_matches([' ', '\t']));
        output.push_str(newline);
    }
    output
}

fn paint(text: &str, style: AnsiStyle, color: bool) -> String {
    if color { format!("{style}{text}{style:#}") } else { text.to_owned() }
}

fn write_progress(stderr: &mut dyn Write, message: &str) {
    // The clear-and-rewrite sequence is emitted only after stderr has been proven to be
    // a terminal. Redirected and machine-oriented workflows never see control bytes.
    let _ = write!(stderr, "\r\x1b[2K{message}").and_then(|()| stderr.flush());
}

fn clear_progress(stderr: &mut dyn Write) {
    let _ = write!(stderr, "\r\x1b[2K").and_then(|()| stderr.flush());
}

/// Classify the result of writing and flushing stdout after the required work succeeded.
fn finish_stdout(result: io::Result<()>, stderr: &mut dyn Write, color: bool) -> Exit {
    match result {
        Ok(()) => Exit::Success,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Exit::Success,
        Err(error) => {
            let label = paint("error:", STYLE_ERROR, color);
            let _ = writeln!(stderr, "{label} failed to write output: {error}");
            Exit::Runtime
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::io::{self, Write};
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{
        Agent, AgentStats, Cli, ColorContext, ColorEnvironment, ColorWhen, Command, Exit,
        ExplicitDialect, Failure, MAX_JOBS, OutputFormat, ReportArgs, ScopeArg, Stats,
        TerminalContext, classify_explicit_source, classify_records, codex_thread_from_locator,
        cursor_composer_ids, decoding_workers, derive_agent_thread_id, execute, ingest_capacity,
        narrow_discoveries, run, run_with_context, stats_requested,
    };
    use clap::Parser;
    use urollup_core::adapters::{AdapterError, claude_project, codex_rollout};
    use urollup_core::ledger::reconcile::ReconcileError;
    use urollup_core::selection::{CurrentSession, Scope, SelectionQuery, SessionIndex};
    use urollup_core::sources::manifest::Representation;
    use urollup_core::sources::{reader, roots};

    /// Classifies one file's records, reading lines of at most `max_line_bytes`.
    fn classify_file(path: &std::path::Path, max_line_bytes: u64) -> Option<Agent> {
        let representation = Representation::of_path(path).expect("a source file name");
        let file = fs::File::open(path).expect("the file opens");
        let mut decoded = reader::decode(file, representation).expect("the decoder starts");
        classify_records(&mut *decoded, max_line_bytes)
    }

    struct Outcome {
        exit: Exit,
        stdout: String,
        stderr: String,
    }

    fn invoke(args: &[&str]) -> Outcome {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command_line = std::iter::once("urollup").chain(args.iter().copied());
        let exit = run(command_line, &mut stdout, &mut stderr);
        Outcome {
            exit,
            stdout: String::from_utf8(stdout).expect("stdout is UTF-8"),
            stderr: String::from_utf8(stderr).expect("stderr is UTF-8"),
        }
    }

    fn invoke_with_context(
        args: Vec<OsString>,
        terminals: TerminalContext,
        environment: ColorEnvironment,
    ) -> Outcome {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_with_context(args, &mut stdout, &mut stderr, terminals, environment, None);
        Outcome {
            exit,
            stdout: String::from_utf8(stdout).expect("stdout is UTF-8"),
            stderr: String::from_utf8(stderr).expect("stderr is UTF-8"),
        }
    }

    fn fixture_report_args(extra: &[&str]) -> Vec<OsString> {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../urollup-core/tests/fixtures/claude-project/nested-null-tool-input");
        ["urollup", "report", "--source"]
            .into_iter()
            .map(OsString::from)
            .chain(std::iter::once(source.into_os_string()))
            .chain(["--no-default-sources"].into_iter().map(OsString::from))
            .chain(extra.iter().copied().map(OsString::from))
            .collect()
    }

    #[test]
    fn mixed_claude_codex_and_cursor_sources_split_agent_and_provider() {
        let fixtures =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
        let mut args = vec![OsString::from("urollup"), OsString::from("report")];
        for source in [
            fixtures.join("claude-project/nested-null-tool-input"),
            fixtures.join("codex-rollout/info-null"),
            fixtures.join("cursor-state/basic"),
        ] {
            args.push(OsString::from("--source"));
            args.push(source.into_os_string());
        }
        args.extend(
            [
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--group-by",
                "agent,provider",
            ]
            .into_iter()
            .map(OsString::from),
        );
        let outcome = invoke_with_context(
            args.clone(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(outcome.exit, Exit::Success, "{}", outcome.stderr);
        let document: serde_json::Value =
            serde_json::from_str(&outcome.stdout).expect("json report");
        let agents: Vec<_> = document["breakdowns"]["agent"]
            .as_array()
            .expect("agent")
            .iter()
            .map(|row| row["value"].as_str().expect("agent value"))
            .collect();
        let providers: Vec<_> = document["breakdowns"]["provider"]
            .as_array()
            .expect("provider")
            .iter()
            .map(|row| row["value"].as_str().expect("provider value"))
            .collect();
        assert_eq!(agents, ["claude", "codex", "cursor"]);
        assert_eq!(providers, ["anthropic", "cursor", "openai"]);
        assert_eq!(document["totals"]["requests"]["owned"], 5);
        assert_eq!(document["diagnostics"][0]["code"], "cursor-estimate-cost");

        args.extend(["--agent", "cursor"].into_iter().map(OsString::from));
        let cursor_only =
            invoke_with_context(args, TerminalContext::default(), ColorEnvironment::default());
        assert_eq!(cursor_only.exit, Exit::Success, "{}", cursor_only.stderr);
        let cursor_document: serde_json::Value =
            serde_json::from_str(&cursor_only.stdout).expect("json report");
        assert_eq!(cursor_document["totals"]["requests"]["owned"], 2);
        let cursor_agents: Vec<_> = cursor_document["breakdowns"]["agent"]
            .as_array()
            .expect("agent")
            .iter()
            .map(|row| row["value"].as_str().expect("agent value"))
            .collect();
        assert_eq!(cursor_agents, ["cursor"]);

        let claude_on_cursor = invoke_with_context(
            {
                let cursor = fixtures.join("cursor-state/basic");
                [
                    "urollup",
                    "report",
                    "--source",
                    cursor.to_str().expect("utf-8 fixture path"),
                    "--no-default-sources",
                    "--format",
                    "json",
                    "--timezone",
                    "UTC",
                    "--agent",
                    "claude",
                ]
                .into_iter()
                .map(OsString::from)
                .collect()
            },
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(claude_on_cursor.exit, Exit::Success, "{}", claude_on_cursor.stderr);
        let empty: serde_json::Value =
            serde_json::from_str(&claude_on_cursor.stdout).expect("json report");
        assert_eq!(empty["totals"]["requests"]["owned"], 0);
        assert!(empty["breakdowns"]["agent"].as_array().is_none_or(Vec::is_empty));

        let cursor = fixtures.join("cursor-state/basic");
        let cursor_source = cursor.to_str().expect("utf-8 fixture path");
        let daily_cursor = invoke_with_context(
            [
                "urollup",
                "daily",
                "--source",
                cursor_source,
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--agent",
                "cursor",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(daily_cursor.exit, Exit::Success, "{}", daily_cursor.stderr);
        let daily_document: serde_json::Value =
            serde_json::from_str(&daily_cursor.stdout).expect("json daily");
        let daily_owned: u64 = daily_document["rows"]
            .as_array()
            .expect("daily rows")
            .iter()
            .map(|row| row["requests"]["owned"].as_u64().unwrap_or(0))
            .sum();
        assert_eq!(daily_owned, 2);

        let daily_claude = invoke_with_context(
            [
                "urollup",
                "daily",
                "--source",
                cursor_source,
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--agent",
                "claude",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(daily_claude.exit, Exit::Success, "{}", daily_claude.stderr);
        let daily_empty: serde_json::Value =
            serde_json::from_str(&daily_claude.stdout).expect("json daily");
        assert!(daily_empty["rows"].as_array().is_none_or(Vec::is_empty));

        let sessions_cursor = invoke_with_context(
            [
                "urollup",
                "sessions",
                "--source",
                cursor_source,
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--agent",
                "cursor",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(sessions_cursor.exit, Exit::Success, "{}", sessions_cursor.stderr);
        let sessions_document: serde_json::Value =
            serde_json::from_str(&sessions_cursor.stdout).expect("json sessions");
        let session_rows = sessions_document["rows"].as_array().expect("session rows");
        assert_eq!(session_rows.len(), 3);
        assert!(session_rows.iter().all(|row| row["agent"] == "cursor"));

        let sessions_claude = invoke_with_context(
            [
                "urollup",
                "sessions",
                "--source",
                cursor_source,
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--agent",
                "claude",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(sessions_claude.exit, Exit::Success, "{}", sessions_claude.stderr);
        let sessions_empty: serde_json::Value =
            serde_json::from_str(&sessions_claude.stdout).expect("json sessions");
        assert!(sessions_empty["rows"].as_array().is_none_or(Vec::is_empty));

        let exact = invoke_with_context(
            [
                "urollup",
                "report",
                "--source",
                cursor_source,
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--session",
                "11111111-1111-4111-8111-111111111111",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(exact.exit, Exit::Success, "{}", exact.stderr);
        let exact_document: serde_json::Value =
            serde_json::from_str(&exact.stdout).expect("json report");
        assert_eq!(exact_document["query"]["selection"], "session");
        assert_eq!(exact_document["totals"]["requests"]["owned"], 1);
        assert_eq!(
            exact_document["breakdowns"]["model"][0]["value"],
            "claude-4.5-opus-high-thinking"
        );

        let subagents = fixtures.join("cursor-state/subagents");
        let parent = "99999999-9999-4999-8999-999999999999";
        let descendants = invoke_with_context(
            [
                "urollup",
                "report",
                "--source",
                subagents.to_str().expect("utf-8 fixture path"),
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--session",
                parent,
                "--group-by",
                "provider,model",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(descendants.exit, Exit::Success, "{}", descendants.stderr);
        let family: serde_json::Value =
            serde_json::from_str(&descendants.stdout).expect("json report");
        assert_eq!(family["query"]["scope"], "descendants");
        assert_eq!(family["totals"]["requests"]["owned"], 3);
        let providers: Vec<_> = family["breakdowns"]["provider"]
            .as_array()
            .expect("provider")
            .iter()
            .map(|row| row["value"].as_str().expect("provider value"))
            .collect();
        assert_eq!(providers, ["anthropic", "cursor", "google"]);

        let self_only = invoke_with_context(
            [
                "urollup",
                "report",
                "--source",
                subagents.to_str().expect("utf-8 fixture path"),
                "--no-default-sources",
                "--format",
                "json",
                "--timezone",
                "UTC",
                "--session",
                parent,
                "--scope",
                "self",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(self_only.exit, Exit::Success, "{}", self_only.stderr);
        let parent_only: serde_json::Value =
            serde_json::from_str(&self_only.stdout).expect("json report");
        assert_eq!(parent_only["query"]["scope"], "self");
        assert_eq!(parent_only["totals"]["requests"]["owned"], 1);
        assert_eq!(parent_only["breakdowns"]["model"][0]["value"], "cursor-grok-4.6-xhigh-fast");
    }

    /// A writer whose every write fails with one error kind.
    struct FailingWriter(io::ErrorKind);

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.0))
        }
    }

    #[test]
    fn exit_codes_follow_the_design_contract() {
        assert_eq!(Exit::Success.code(), 0);
        assert_eq!(Exit::Runtime.code(), 1);
        assert_eq!(Exit::Usage.code(), 2);
    }

    #[test]
    fn version_goes_to_stdout_and_matches_the_core() {
        let outcome = invoke(&["--version"]);
        assert_eq!(outcome.exit, Exit::Success);
        assert_eq!(outcome.stdout, format!("urollup {}\n", urollup_core::VERSION));
        assert_eq!(outcome.stderr, "");
    }

    #[test]
    fn help_goes_to_stdout_and_lists_the_commands() {
        let outcome = invoke(&["--help"]);
        assert_eq!(outcome.exit, Exit::Success);
        for command in ["report", "daily", "sessions"] {
            assert!(outcome.stdout.contains(command), "help lacks {command}:\n{}", outcome.stdout);
        }
        assert_eq!(outcome.stderr, "");
    }

    #[test]
    fn automatic_color_follows_the_destination_stream() {
        let help = invoke_with_context(
            ["urollup", "--help"].into_iter().map(OsString::from).collect(),
            TerminalContext { stdout_is_terminal: true, stderr_is_terminal: false },
            ColorEnvironment::default(),
        );
        assert!(help.stdout.contains("\u{1b}["), "{:?}", help.stdout);
        assert_eq!(help.stderr, "");

        let redirected = invoke_with_context(
            ["urollup", "--help"].into_iter().map(OsString::from).collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert!(!redirected.stdout.contains("\u{1b}["), "{:?}", redirected.stdout);

        let error = invoke_with_context(
            ["urollup", "not-a-command"].into_iter().map(OsString::from).collect(),
            TerminalContext { stdout_is_terminal: false, stderr_is_terminal: true },
            ColorEnvironment::default(),
        );
        assert!(error.stderr.contains("\u{1b}["), "{:?}", error.stderr);
        assert_eq!(error.stdout, "");
    }

    #[test]
    fn explicit_and_environment_color_controls_have_stable_precedence() {
        let always = invoke_with_context(
            ["urollup", "--color", "always", "--help"].into_iter().map(OsString::from).collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert!(always.stdout.contains("\u{1b}["), "{:?}", always.stdout);

        let no_color = invoke_with_context(
            ["urollup", "--color", "always", "--help"].into_iter().map(OsString::from).collect(),
            TerminalContext { stdout_is_terminal: true, stderr_is_terminal: true },
            ColorEnvironment { no_color: true, force_color: true },
        );
        assert!(!no_color.stdout.contains("\u{1b}["), "{:?}", no_color.stdout);

        let forced = invoke_with_context(
            ["urollup", "--help"].into_iter().map(OsString::from).collect(),
            TerminalContext::default(),
            ColorEnvironment { no_color: false, force_color: true },
        );
        assert!(forced.stdout.contains("\u{1b}["), "{:?}", forced.stdout);

        let never = ColorContext {
            when: ColorWhen::Never,
            machine: false,
            environment: ColorEnvironment { no_color: false, force_color: true },
        };
        assert!(!never.enabled(true));
    }

    #[test]
    fn machine_output_is_plain_and_suppresses_progress() {
        let outcome = invoke_with_context(
            fixture_report_args(&["--format", "json", "--color", "always"]),
            TerminalContext { stdout_is_terminal: true, stderr_is_terminal: true },
            ColorEnvironment { no_color: false, force_color: true },
        );
        assert_eq!(outcome.exit, Exit::Success);
        assert!(!outcome.stdout.contains("\u{1b}["), "{:?}", outcome.stdout);
        assert_eq!(outcome.stderr, "");
        serde_json::from_str::<serde_json::Value>(&outcome.stdout).expect("plain JSON output");
    }

    #[test]
    fn terminal_tables_share_the_color_policy() {
        let terminals = TerminalContext { stdout_is_terminal: true, stderr_is_terminal: false };
        let colored =
            invoke_with_context(fixture_report_args(&[]), terminals, ColorEnvironment::default());
        assert_eq!(colored.exit, Exit::Success);
        assert!(colored.stdout.contains("\u{1b}["), "{:?}", colored.stdout);
        assert_eq!(colored.stderr, "");

        let plain = invoke_with_context(
            fixture_report_args(&["--color", "always"]),
            terminals,
            ColorEnvironment { no_color: true, force_color: true },
        );
        assert_eq!(plain.exit, Exit::Success);
        assert!(!plain.stdout.contains("\u{1b}["), "{:?}", plain.stdout);
        assert_eq!(plain.stderr, "");
    }

    #[test]
    fn interactive_table_progress_is_stderr_only_and_cleans_up() {
        let terminals = TerminalContext { stdout_is_terminal: true, stderr_is_terminal: true };
        let outcome = invoke_with_context(
            fixture_report_args(&["--color", "never"]),
            terminals,
            ColorEnvironment::default(),
        );
        assert_eq!(outcome.exit, Exit::Success);
        assert_eq!(outcome.stderr, "\r\u{1b}[2KReading usage logs…\r\u{1b}[2K");
        assert!(outcome.stdout.starts_with("urollup report\n"), "{}", outcome.stdout);
        assert!(!outcome.stdout.contains("Reading usage logs"));

        let disabled = invoke_with_context(
            fixture_report_args(&["--no-progress", "--color", "never"]),
            terminals,
            ColorEnvironment::default(),
        );
        assert_eq!(disabled.exit, Exit::Success);
        assert_eq!(disabled.stderr, "");

        let redirected = invoke_with_context(
            fixture_report_args(&["--color", "never"]),
            TerminalContext { stdout_is_terminal: true, stderr_is_terminal: false },
            ColorEnvironment::default(),
        );
        assert_eq!(redirected.exit, Exit::Success);
        assert_eq!(redirected.stderr, "");
    }

    #[test]
    fn progress_cleans_up_before_runtime_diagnostics() {
        let outcome = invoke_with_context(
            [
                "urollup",
                "report",
                "--source",
                "/urollup-test/missing-source",
                "--no-default-sources",
                "--color",
                "never",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext { stdout_is_terminal: true, stderr_is_terminal: true },
            ColorEnvironment::default(),
        );
        assert_eq!(outcome.exit, Exit::Runtime);
        assert_eq!(outcome.stdout, "");
        assert!(
            outcome.stderr.starts_with("\r\u{1b}[2KReading usage logs…\r\u{1b}[2Kerror:"),
            "{:?}",
            outcome.stderr
        );
    }

    #[test]
    fn a_bare_invocation_is_a_usage_error_on_stderr() {
        let outcome = invoke(&[]);
        assert_eq!(outcome.exit, Exit::Usage);
        assert_eq!(outcome.stdout, "");
        assert!(outcome.stderr.contains("Usage: urollup"), "{}", outcome.stderr);
    }

    #[test]
    fn an_unknown_command_is_a_usage_error_on_stderr() {
        let outcome = invoke(&["weekly-ish"]);
        assert_eq!(outcome.exit, Exit::Usage);
        assert_eq!(outcome.stdout, "");
        assert!(outcome.stderr.starts_with("error:"), "{}", outcome.stderr);
    }

    #[test]
    fn report_accepts_milestone_selection_flags() {
        let cli = Cli::try_parse_from([
            "urollup",
            "report",
            "--current",
            "--session",
            "native-one",
            "--session",
            "thr-two",
            "--all",
            "--scope",
            "descendants",
            "--source",
            "logs",
            "--no-default-sources",
        ])
        .expect("selection flags parse");
        let Command::Report(ReportArgs { selection, group_by }) = cli.command else {
            panic!("expected report command");
        };
        assert!(group_by.is_empty());
        assert!(selection.current);
        assert_eq!(selection.sessions, ["native-one", "thr-two"]);
        assert!(selection.all);
        assert_eq!(selection.scope, Some(ScopeArg::Descendants));
        assert_eq!(selection.sources, [PathBuf::from("logs")]);
        assert!(selection.no_default_sources);
        assert_eq!(selection.format, OutputFormat::Table);
    }

    #[test]
    fn every_report_command_exposes_selection_help() {
        for command in ["report", "daily", "sessions"] {
            let outcome = invoke(&[command, "--help"]);
            assert_eq!(outcome.exit, Exit::Success, "{command}");
            for flag in [
                "--current",
                "--session",
                "--all",
                "--scope",
                "--source",
                "--no-default-sources",
                "--format",
                "--timezone",
                "--agent",
                "--color",
                "--no-progress",
            ] {
                assert!(outcome.stdout.contains(flag), "{command} help lacks {flag}");
            }
        }
    }

    #[test]
    fn daily_and_sessions_reject_group_by_as_a_usage_error() {
        // Joint calendar and dimension grouping is not implemented (uro-qvp1), so a
        // grouping flag on a rollup must fail rather than print ungrouped rows.
        for command in ["daily", "sessions"] {
            let outcome = invoke(&[
                command,
                "--all",
                "--no-default-sources",
                "--timezone",
                "UTC",
                "--group-by",
                "model",
            ]);
            assert_eq!(outcome.exit, Exit::Usage, "{command}: {}", outcome.stdout);
            assert_eq!(outcome.stdout, "", "{command}");
            assert!(
                outcome.stderr.starts_with("error: unexpected argument '--group-by' found"),
                "{command}: {}",
                outcome.stderr
            );
        }
    }

    #[test]
    fn help_describes_report_groups_as_separate_breakdowns_without_tools() {
        let root = invoke(&["--help"]);
        assert!(!root.stdout.contains("tools"), "{}", root.stdout);
        let report = invoke(&["report", "--help"]);
        assert_eq!(report.exit, Exit::Success);
        assert!(!report.stdout.contains("tools"), "{}", report.stdout);
        assert!(report.stdout.contains("--group-by"), "{}", report.stdout);
        assert!(report.stdout.contains("never combined"), "{}", report.stdout);
        for command in ["daily", "sessions"] {
            let help = invoke(&[command, "--help"]);
            assert_eq!(help.exit, Exit::Success, "{command}");
            assert!(!help.stdout.contains("--group-by"), "{command}: {}", help.stdout);
        }
    }

    #[test]
    fn explicit_sources_detect_fixture_roots_and_compressed_artifacts() {
        let fixtures =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
        let claude =
            classify_explicit_source(&fixtures.join("claude-project/nested-null-tool-input"))
                .expect("Claude fixture is detected");
        assert!(matches!(claude.as_slice(), [ExplicitDialect::Claude(_)]));

        let compressed = fixtures.join(
            "codex-rollout/zst-twin/sessions/2026/09/08/\
             rollout-2026-09-08T06-00-00-019f0000-0000-7000-8000-001000000001.jsonl.zst",
        );
        let codex = classify_explicit_source(&compressed).expect("Codex artifact is detected");
        assert!(matches!(codex.as_slice(), [ExplicitDialect::Codex(path)] if path == &compressed));
    }

    #[test]
    fn cursor_jsonl_is_recognized_and_rejected_as_a_usage_source() {
        let root = tempfile::tempdir().unwrap();
        let transcript = root.path().join("agent-transcript.jsonl");
        fs::write(
            &transcript,
            concat!(
                r#"{"role":"assistant","message":{"content":[{"type":"text","text":"ok"}]}}"#,
                "\n",
                r#"{"type":"turn_ended","status":"success","error":null}"#,
                "\n",
            ),
        )
        .unwrap();
        assert_eq!(classify_file(&transcript, 1024), Some(Agent::Cursor));
        let error = classify_explicit_source(&transcript).unwrap_err();
        assert_eq!(error.exit, Exit::Runtime);
        assert!(error.message.contains("not a usage owner"), "{}", error.message);

        let claude_shaped = root.path().join("claude.jsonl");
        fs::write(&claude_shaped, "{\"type\":\"assistant\",\"sessionId\":\"cccc\"}\n").unwrap();
        assert_eq!(classify_file(&claude_shaped, 1024), Some(Agent::Claude));
    }

    #[test]
    fn cursor_snapshot_plus_jsonl_source_is_rejected() {
        let fixtures =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
        let snapshot = fixtures.join("cursor-state/basic/cursor-state.json");
        let transcript = fixtures.join(
            "cursor-state/basic/agent-transcripts/11111111-1111-4111-8111-111111111111/11111111-1111-4111-8111-111111111111.jsonl",
        );
        let outcome = invoke_with_context(
            [
                "urollup",
                "report",
                "--source",
                snapshot.to_str().expect("utf-8 snapshot"),
                "--source",
                transcript.to_str().expect("utf-8 transcript"),
                "--no-default-sources",
                "--all",
                "--timezone",
                "UTC",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            TerminalContext::default(),
            ColorEnvironment::default(),
        );
        assert_eq!(outcome.exit, Exit::Runtime, "{}", outcome.stderr);
        assert!(outcome.stderr.contains("not a usage owner"), "{}", outcome.stderr);
    }

    #[test]
    fn explicit_artifact_classification_bounds_plain_and_decoded_compressed_lines() {
        let root = tempfile::tempdir().unwrap();
        let line =
            format!("{}\n", serde_json::json!({"type": "assistant", "padding": "x".repeat(128)}));
        let plain = root.path().join("oversized.jsonl");
        fs::write(&plain, &line).unwrap();
        let compressed = root.path().join("oversized.jsonl.zst");
        fs::write(&compressed, zstd::stream::encode_all(line.as_bytes(), 1).unwrap()).unwrap();
        let gzipped = root.path().join("oversized.jsonl.gz");
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(line.as_bytes()).unwrap();
        fs::write(&gzipped, encoder.finish().unwrap()).unwrap();

        for path in [plain, compressed, gzipped] {
            // A record longer than the bound is not buffered, so it decides nothing.
            assert_eq!(classify_file(&path, 32), None, "{}", path.display());
            assert_eq!(classify_file(&path, 1024), Some(Agent::Claude), "{}", path.display());
        }
    }

    #[test]
    fn explicit_claude_root_ignores_transcripts_without_dialect_records() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("aaaa.jsonl"),
            "{\"type\":\"unknown-title\",\"sessionId\":\"aaaa\"}\n",
        )
        .unwrap();
        fs::write(
            project.join("bbbb.jsonl"),
            "{\"type\":\"custom-title\",\"sessionId\":\"bbbb\"}\n",
        )
        .unwrap();
        fs::write(project.join("cccc.jsonl"), "{\"type\":\"user\",\"sessionId\":\"cccc\"}\n")
            .unwrap();

        let classified = classify_explicit_source(&project).unwrap();
        assert!(
            matches!(classified.as_slice(), [ExplicitDialect::Claude(path)] if path == &project)
        );
        assert_eq!(classify_file(&project.join("aaaa.jsonl"), 1024), None);
        assert_eq!(classify_file(&project.join("bbbb.jsonl"), 1024), Some(Agent::Claude));
    }

    #[test]
    fn cursor_composer_ids_take_uuids_and_ignore_whole_store_reports() {
        assert_eq!(
            cursor_composer_ids(&SelectionQuery { all: true, ..SelectionQuery::default() }),
            None
        );
        let current = CurrentSession {
            agent: Agent::Cursor,
            selector: OsString::from("11111111-1111-4111-8111-111111111111"),
        };
        assert_eq!(
            cursor_composer_ids(&SelectionQuery {
                current: Some(current),
                ..SelectionQuery::default()
            }),
            Some(vec!["11111111-1111-4111-8111-111111111111".to_owned()])
        );
        assert_eq!(
            cursor_composer_ids(&SelectionQuery {
                sessions: vec!["not-a-composer".into()],
                ..SelectionQuery::default()
            }),
            None
        );
        assert_eq!(
            cursor_composer_ids(&SelectionQuery {
                sessions: vec![OsString::from(
                    "/tmp/agent-transcripts/11111111-1111-4111-8111-111111111111/11111111-1111-4111-8111-111111111111.jsonl"
                )],
                ..SelectionQuery::default()
            }),
            Some(vec!["11111111-1111-4111-8111-111111111111".to_owned()])
        );
    }

    #[test]
    fn exact_claude_selection_prunes_unrelated_sessions_before_ingestion() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("projects/project");
        let subagents = project.join("selected-session/subagents");
        fs::create_dir_all(&subagents).unwrap();
        fs::write(
            project.join("selected-session.jsonl"),
            concat!(
                r#"{"type":"assistant","uuid":"uuid-main","sessionId":"selected-session","requestId":"request-main","cwd":"/workspace/project","timestamp":"2026-09-16T12:00:00Z","message":{"id":"message-main","model":"claude-test","usage":{"input_tokens":2,"output_tokens":3},"content":[]}}"#,
                "\n"
            ),
        )
        .unwrap();
        fs::write(
            subagents.join("agent-child.jsonl"),
            concat!(
                r#"{"type":"assistant","uuid":"uuid-child","sessionId":"selected-session","requestId":"request-child","cwd":"/workspace/project","timestamp":"2026-09-16T12:01:00Z","message":{"id":"message-child","model":"claude-test","usage":{"input_tokens":5,"output_tokens":7},"content":[]}}"#,
                "\n"
            ),
        )
        .unwrap();
        // A third request in an unrelated session would appear if selection read it.
        fs::write(
            project.join("unrelated-session.jsonl"),
            concat!(
                r#"{"type":"assistant","uuid":"uuid-other","sessionId":"unrelated-session","requestId":"request-other","cwd":"/workspace/project","timestamp":"2026-09-16T12:02:00Z","message":{"id":"message-other","model":"claude-test","usage":{"input_tokens":11,"output_tokens":13},"content":[]}}"#,
                "\n"
            ),
        )
        .unwrap();

        let original = roots::discover(&[root.path().join("projects")]);
        assert_eq!(original.sources.len(), 3);
        let mut analytical_claude = original.clone();
        let mut analytical_codex = roots::Discovery::default();
        let analytical_query = SelectionQuery {
            sessions: vec![OsString::from(
                derive_agent_thread_id(Agent::Claude, "selected-session").unwrap().to_string(),
            )],
            scope: Some(Scope::Descendants),
            ..SelectionQuery::default()
        };
        assert!(
            narrow_discoveries(&mut analytical_claude, &mut analytical_codex, &analytical_query)
                .unwrap()
        );
        assert_eq!(analytical_claude.sources.len(), 2);

        let mut claude = original;
        let mut codex = roots::Discovery::default();
        let query = SelectionQuery {
            current: Some(CurrentSession {
                agent: Agent::Claude,
                selector: OsString::from("selected-session"),
            }),
            scope: Some(Scope::Descendants),
            ..SelectionQuery::default()
        };

        assert!(narrow_discoveries(&mut claude, &mut codex, &query).unwrap());
        assert_eq!(claude.sources.len(), 2);

        let ingested = claude_project::ingest_discovery(claude, false).unwrap();
        let mut index = SessionIndex::default();
        index.add(Agent::Claude, &ingested).unwrap();
        let selected = index.select(&query).unwrap();
        assert_eq!(selected.len(), 2, "the main session brings its child into scope");
        assert_eq!(ingested.ledger.requests.len(), 2);
    }

    #[test]
    fn codex_native_and_path_selection_keep_the_whole_session_family() {
        let fixtures =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
        let root = fixtures.join("codex-rollout/paginated-subagent");
        let codex_roots = codex_rollout::rollout_roots(&[root]);
        let original = roots::discover(&codex_roots);
        assert_eq!(original.sources.len(), 2);
        let parent = OsString::from("019f0000-0000-7000-8000-000700000001");

        for selector in [
            parent.clone(),
            OsString::from(
                derive_agent_thread_id(Agent::Codex, parent.to_str().unwrap()).unwrap().to_string(),
            ),
            original.sources[0]
                .files
                .primary()
                .expect("fixture has a primary representation")
                .0
                .as_os_str()
                .to_owned(),
        ] {
            let mut claude = roots::Discovery::default();
            let mut codex = original.clone();
            let query = SelectionQuery {
                sessions: vec![selector],
                scope: Some(Scope::Descendants),
                ..SelectionQuery::default()
            };
            assert!(narrow_discoveries(&mut claude, &mut codex, &query).unwrap());
            assert_eq!(codex.sources.len(), 2);
            let ingested = codex_rollout::ingest_discovery(codex, true).unwrap();
            let mut index = SessionIndex::default();
            index.add(Agent::Codex, &ingested).unwrap();
            assert_eq!(index.select(&query).unwrap().len(), 2);
        }
    }

    #[test]
    fn codex_catalog_reads_compressed_session_headers() {
        let fixtures =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
        let compressed = fixtures.join(
            "codex-rollout/zst-twin/sessions/2026/09/08/\
             rollout-2026-09-08T06-00-00-019f0000-0000-7000-8000-001000000001.jsonl.zst",
        );
        let mut claude = roots::Discovery::default();
        let mut codex = roots::discover(std::slice::from_ref(&compressed));
        let query = SelectionQuery {
            sessions: vec![OsString::from("019f0000-0000-7000-8000-001000000001")],
            ..SelectionQuery::default()
        };

        assert!(narrow_discoveries(&mut claude, &mut codex, &query).unwrap());
        assert_eq!(codex.sources.len(), 1);
        codex_rollout::ingest_discovery(codex, true).unwrap();
    }

    #[test]
    fn codex_catalog_connects_parent_and_fork_edges_without_session_id() {
        let root = tempfile::tempdir().unwrap();
        let sessions = root.path().join("sessions/2026/09/16");
        fs::create_dir_all(&sessions).unwrap();
        let parent = "019f0000-0000-7000-8000-002000000001";
        let child = "019f0000-0000-7000-8000-002000000002";
        let fork = "019f0000-0000-7000-8000-002000000003";
        let write_meta = |name: &str, payload: serde_json::Value| {
            fs::write(
                sessions.join(name),
                format!(
                    "{}\n",
                    serde_json::json!({
                        "timestamp": "2026-09-16T12:00:00Z",
                        "type": "session_meta",
                        "payload": payload,
                    })
                ),
            )
            .unwrap();
        };
        write_meta(
            &format!("rollout-2026-09-16T12-00-00-{parent}.jsonl"),
            serde_json::json!({"id": parent, "cwd": "/workspace/project"}),
        );
        write_meta(
            &format!("rollout-2026-09-16T12-01-00-{child}.jsonl"),
            serde_json::json!({
                "id": child,
                "parent_thread_id": parent,
                "cwd": "/workspace/project",
            }),
        );
        write_meta(
            &format!("rollout-2026-09-16T12-02-00-{fork}.jsonl"),
            serde_json::json!({
                "id": fork,
                "forked_from_id": child,
                "cwd": "/workspace/project",
            }),
        );

        let mut claude = roots::Discovery::default();
        let codex_roots = codex_rollout::rollout_roots(&[root.path().to_owned()]);
        let mut codex = roots::discover(&codex_roots);
        let query = SelectionQuery {
            sessions: vec![OsString::from(parent)],
            scope: Some(Scope::Descendants),
            ..SelectionQuery::default()
        };

        assert!(narrow_discoveries(&mut claude, &mut codex, &query).unwrap());
        assert_eq!(codex.sources.len(), 3, "spawn and fork links form one support family");
        let ingested = codex_rollout::ingest_discovery(codex, true).unwrap();
        let mut index = SessionIndex::default();
        index.add(Agent::Codex, &ingested).unwrap();
        assert_eq!(
            index.select(&query).unwrap().len(),
            2,
            "fork support is ingested but fork is not a descendant"
        );
    }

    #[test]
    fn codex_catalog_keeps_rollouts_whose_discovered_header_cannot_be_read() {
        let root = tempfile::tempdir().unwrap();
        let sessions = root.path().join("sessions/2026/09/16");
        fs::create_dir_all(&sessions).unwrap();
        let thread = |n: u32| format!("019f0000-0000-7000-8000-0030000000{n:02}");
        let rollout =
            |n: u32| sessions.join(format!("rollout-2026-09-16T12-00-00-{}.jsonl", thread(n)));
        let header = |n: u32, parent: Option<u32>| {
            let mut payload = serde_json::json!({"id": thread(n), "cwd": "/workspace/project"});
            if let Some(parent) = parent {
                payload["parent_thread_id"] = serde_json::json!(thread(parent));
            }
            format!(
                "{}\n",
                serde_json::json!({
                    "timestamp": "2026-09-16T12:00:00Z",
                    "type": "session_meta",
                    "payload": payload,
                })
            )
        };
        // 1 has children 2 and 6, and 2 has child 3; 4 is unrelated.
        for (n, parent) in [(1, None), (2, Some(1)), (3, Some(2)), (4, None), (6, Some(1))] {
            fs::write(rollout(n), header(n, parent)).unwrap();
        }
        // An interrupted compressor left nothing readable.
        fs::write(rollout(5).with_extension("jsonl.gz"), b"").unwrap();

        let codex_roots = codex_rollout::rollout_roots(&[root.path().to_owned()]);
        let discovered = roots::discover(&codex_roots);
        assert_eq!(discovered.sources.len(), 6);
        // After discovery `zstd --rm` compresses 2, whose header alone links 1 to 3, and 6
        // expires.
        fs::write(
            rollout(2).with_extension("jsonl.zst"),
            zstd::stream::encode_all(header(2, Some(1)).as_bytes(), 1).unwrap(),
        )
        .unwrap();
        fs::remove_file(rollout(2)).unwrap();
        fs::remove_file(rollout(6)).unwrap();

        let mut claude = roots::Discovery::default();
        let mut codex = discovered;
        let query = SelectionQuery {
            sessions: vec![OsString::from(thread(1))],
            scope: Some(Scope::Descendants),
            ..SelectionQuery::default()
        };
        assert!(narrow_discoveries(&mut claude, &mut codex, &query).unwrap());
        let kept: Vec<String> =
            codex.sources.iter().map(|source| codex_thread_from_locator(&source.locator)).collect();
        assert_eq!(
            kept,
            [thread(1), thread(2), thread(3), thread(5), thread(6)],
            "the compressed child's header is read from its new file, and rollouts with no \
             readable header are left for ingest to read or report"
        );
    }

    #[test]
    fn jobs_default_when_unset_and_accept_whole_numbers_in_range() {
        let default = urollup_core::sources::parallel::default_workers();
        assert_eq!(decoding_workers(None).unwrap(), default);
        assert_eq!(decoding_workers(Some(OsStr::new(""))).unwrap(), default);
        for (value, expected) in [("1", 1), ("8", 8), ("012", 12), ("256", MAX_JOBS)] {
            assert_eq!(decoding_workers(Some(OsStr::new(value))).unwrap().get(), expected);
        }
    }

    #[test]
    fn jobs_outside_the_range_or_not_whole_numbers_are_usage_errors() {
        for value in ["0", "257", "-1", "+4", " 4", "4 ", "1.5", "four", "99999999999999999999999"]
        {
            let failure = decoding_workers(Some(OsStr::new(value))).unwrap_err();
            assert_eq!(failure.exit, Exit::Usage, "{value:?}");
            assert!(
                failure.message.starts_with("UROLLUP_JOBS must be a whole number"),
                "{value:?}"
            );
            assert!(failure.message.contains(&format!("{value:?}")), "{}", failure.message);
        }
    }

    #[test]
    fn stats_are_off_unless_the_variable_is_one() {
        assert!(!stats_requested(None).unwrap());
        assert!(!stats_requested(Some(OsStr::new(""))).unwrap());
        assert!(!stats_requested(Some(OsStr::new("0"))).unwrap());
        assert!(stats_requested(Some(OsStr::new("1"))).unwrap());
    }

    #[test]
    fn other_stats_values_are_usage_errors() {
        for value in ["2", "01", " 1", "1 ", "true", "yes"] {
            let failure = stats_requested(Some(OsStr::new(value))).unwrap_err();
            assert_eq!(failure.exit, Exit::Usage, "{value:?}");
            assert!(failure.message.starts_with("UROLLUP_STATS must be 1"), "{value:?}");
            assert!(failure.message.contains(&format!("{value:?}")), "{}", failure.message);
        }
    }

    #[test]
    fn stats_render_phases_workers_and_counts_as_key_value_lines() {
        let stats = Stats {
            workers: NonZeroUsize::new(8),
            phases: vec![
                ("discovery", Duration::from_millis(41)),
                ("claude_ingest", Duration::from_micros(11_203_400)),
            ],
            agents: vec![AgentStats {
                agent: "claude",
                sources: 2920,
                observations: 350_112,
                requests: 176_634,
                limit_observations: 3,
                diagnostics: 7,
            }],
        };
        assert_eq!(
            stats.render(Duration::from_millis(11_250)),
            concat!(
                "stats: workers=8\n",
                "stats: phase=discovery seconds=0.041\n",
                "stats: phase=claude_ingest seconds=11.203\n",
                "stats: agent=claude sources=2920 observations=350112 requests=176634 ",
                "limit_observations=3 diagnostics=7\n",
                "stats: total seconds=11.250\n",
            )
        );
        assert_eq!(Stats::default().render(Duration::ZERO), "stats: total seconds=0.000\n");
    }

    #[test]
    fn a_command_records_every_phase_and_both_agents() {
        let cli = Cli::try_parse_from(fixture_report_args(&["--format", "json"])).unwrap();
        let mut stats = Stats::default();
        execute(&cli.command, false, &mut stats).unwrap();

        assert!(stats.workers.is_some());
        let phases: Vec<_> = stats.phases.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            phases,
            [
                "discovery",
                "codex_ingest",
                "claude_ingest",
                "cursor_ingest",
                "session_index",
                "query_render"
            ]
        );
        let [codex, claude, cursor] = stats.agents.as_slice() else {
            panic!("expected one row per agent: {:?}", stats.agents);
        };
        assert_eq!((claude.agent, codex.agent, cursor.agent), ("claude", "codex", "cursor"));
        assert!(claude.sources > 0 && claude.requests > 0, "{claude:?}");
        assert!(claude.observations >= u64::try_from(claude.requests).unwrap(), "{claude:?}");
        assert_eq!(codex.sources, 0);
        assert_eq!(codex.observations, 0);
        assert_eq!(cursor.sources, 0);
        assert_eq!(cursor.observations, 0);
    }

    #[test]
    fn explicit_capacity_does_not_probe_physical_memory() {
        for (ram, rows, env) in [
            (None, Some(10), None),
            (Some("1G"), None, None),
            (Some("1G"), Some(10), None),
            (None, None, Some(OsStr::new("512M"))),
            (Some("1G"), None, Some(OsStr::new("25%"))),
        ] {
            let capacity = ingest_capacity(ram, rows, env, || {
                panic!("explicit capacity must not invoke a potentially slow RAM probe")
            });
            assert!(capacity.is_ok());
        }
    }

    #[test]
    fn ingest_capacity_defaults_to_25_percent_or_2_gib_fallback() {
        let fallback = ingest_capacity(None, None, None, || None).unwrap();
        assert_eq!(fallback.label(), "2 GiB");
        let ram = ingest_capacity(None, None, None, || Some(32 << 30)).unwrap();
        assert_eq!(ram.label(), "25% of RAM (8 GiB)");
    }

    #[test]
    fn ingest_capacity_parses_sizes_percents_and_prefers_the_stricter_control() {
        let eight = ingest_capacity(Some("8G"), None, None, || Some(32 << 30)).unwrap();
        assert_eq!(eight.label(), "8 GiB");
        let rows = ingest_capacity(Some("8G"), Some(10), None, || Some(32 << 30)).unwrap();
        assert_eq!(rows.label(), "10 rows");
        let override_rows = ingest_capacity(None, Some(10), None, || Some(32 << 30)).unwrap();
        assert_eq!(override_rows.label(), "10 rows");
        let flag_beats_env =
            ingest_capacity(Some("1G"), None, Some(OsStr::new("8G")), || None).unwrap();
        assert_eq!(flag_beats_env.label(), "1 GiB");
        let env = ingest_capacity(None, None, Some(OsStr::new("512M")), || None).unwrap();
        assert_eq!(env.label(), "512 MiB");
        let unknown_percent = ingest_capacity(Some("25%"), None, None, || None).unwrap_err();
        assert_eq!(unknown_percent.exit, Exit::Usage);
        assert!(unknown_percent.message.contains("physical RAM is unknown"), "{unknown_percent:?}");
    }

    #[test]
    fn a_tiny_max_rows_ceiling_fails_a_fixture_with_the_capacity_diagnostic() {
        let cli =
            Cli::try_parse_from(fixture_report_args(&["--max-rows", "1", "--format", "json"]))
                .unwrap();
        let error = execute(&cli.command, false, &mut Stats::default()).unwrap_err();
        assert_eq!(error.exit, Exit::Runtime);
        assert!(
            error.message.contains("exceed the reconciliation capacity of 1 compact rows (1 rows)"),
            "{}",
            error.message
        );
        assert!(
            error.message.contains("pass narrower --source roots with --no-default-sources"),
            "{}",
            error.message
        );
    }

    #[test]
    fn a_capacity_failure_exits_one_and_suggests_narrower_sources() {
        let error = AdapterError::Reconcile(ReconcileError::CapacityExceeded {
            observations: 9_000_001,
            maximum: 9_000_000,
            limit: "8 GiB".into(),
        });
        let failure = Failure::adapter(&error);
        assert_eq!(failure.exit, Exit::Runtime);
        assert_eq!(
            failure.message,
            "9000001 request observations exceed the reconciliation capacity of 9000000 compact \
             rows (8 GiB); pass narrower --source roots with --no-default-sources"
        );
    }

    #[test]
    fn a_closed_stdout_after_complete_output_is_success() {
        let mut stderr = Vec::new();
        let mut stdout = FailingWriter(io::ErrorKind::BrokenPipe);
        let exit = run(["urollup", "--version"], &mut stdout, &mut stderr);
        assert_eq!(exit, Exit::Success);
        assert!(stderr.is_empty());
    }

    #[test]
    fn any_other_stdout_write_failure_is_a_runtime_failure() {
        let mut stderr = Vec::new();
        let mut stdout = FailingWriter(io::ErrorKind::Other);
        let exit = run(["urollup", "--version"], &mut stdout, &mut stderr);
        assert_eq!(exit, Exit::Runtime);
        let diagnostic = String::from_utf8(stderr).expect("stderr is UTF-8");
        assert!(diagnostic.starts_with("error: failed to write output"), "{diagnostic}");
    }

    #[test]
    fn a_closed_stderr_does_not_change_the_exit_class() {
        let mut stdout = Vec::new();
        let mut stderr = FailingWriter(io::ErrorKind::BrokenPipe);
        assert_eq!(run(["urollup", "not-a-command"], &mut stdout, &mut stderr), Exit::Usage);
        assert_eq!(run(["urollup"], &mut stdout, &mut stderr), Exit::Usage);
    }
}
