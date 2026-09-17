//! Argument parsing, stream handling and exit translation for the `urollup` executable.
//!
//! [`run_with_context`] takes stdout and stderr as injected writers plus explicit
//! terminal capabilities, so stream and exit behavior is unit-testable without spawning
//! a process. stdout carries only requested data (help and version text count as
//! requested); every diagnostic goes to stderr.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::builder::styling::{AnsiColor, Style as AnsiStyle, Styles};
use clap::{Args, ColorChoice, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use urollup_core::adapters::discovery::DiscoveryEnvironment;
use urollup_core::adapters::{AdapterError, Ingested, claude_project, codex_rollout};
use urollup_core::ledger::reconcile::ReconcileError;
use urollup_core::query::{
    GroupBy, QueryMetadata, QuerySource, ResolvedTimeZone, daily, report, sessions,
};
use urollup_core::selection::{
    Agent, CurrentEnvironment, Scope, SelectionError, SelectionQuery, SessionIndex,
    derive_agent_thread_id,
};
use urollup_core::sources::manifest::Representation;
use urollup_core::sources::parallel;
use urollup_core::sources::reader::ReadOptions;
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
const STATS_VARIABLE: &str = "UROLLUP_STATS";
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
    /// Session report: totals, breakdowns, sizes, tools and limitations
    Report(SelectionArgs),
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
            Self::Report(args) | Self::Daily(args) | Self::Sessions(args) => args,
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

    /// Report breakdown; repeatable and comma-delimited
    #[arg(long, value_enum, value_delimiter = ',', value_name = "DIMENSION")]
    group_by: Vec<GroupByArg>,

    /// Add a source root or JSONL artifact; repeatable
    #[arg(long = "source", value_name = "PATH")]
    sources: Vec<PathBuf>,

    /// Read only paths named by --source
    #[arg(long)]
    no_default_sources: bool,
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
}

impl From<GroupByArg> for GroupBy {
    fn from(value: GroupByArg) -> Self {
        match value {
            GroupByArg::Project => Self::Project,
            GroupByArg::Account => Self::Account,
            GroupByArg::Model => Self::Model,
            GroupByArg::Effort => Self::Effort,
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
    run_with_context(args, stdout, stderr, TerminalContext::default(), ColorEnvironment::default())
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
    let show_stats = match stats_requested(std::env::var_os(STATS_VARIABLE).as_deref()) {
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
        for source in &args.sources {
            for explicit in classify_explicit_source(source)? {
                match explicit {
                    ExplicitDialect::Claude(root) => claude_roots.push(root),
                    ExplicitDialect::Codex(root) => codex_roots.push(root),
                }
            }
        }
        let mut claude_discovery = roots::discover(&claude_roots);
        let codex_roots = codex_rollout::rollout_roots(&codex_roots);
        let mut codex_discovery = roots::discover(&codex_roots);
        narrow_discoveries(&mut claude_discovery, &mut codex_discovery, query)?;
        stats.phase("discovery", started);

        let started = Instant::now();
        let claude = claude_project::ingest_discovery_with_workers(
            claude_discovery,
            claude_missing_is_error,
            workers,
        )
        .map_err(|error| Failure::adapter(&error))?;
        stats.phase("claude_ingest", started);
        stats.agent("claude", &claude);

        let started = Instant::now();
        let codex = codex_rollout::ingest_discovery_with_workers(
            codex_discovery,
            codex_missing_is_error,
            workers,
        )
        .map_err(|error| Failure::adapter(&error))?;
        stats.phase("codex_ingest", started);
        stats.agent("codex", &codex);

        let started = Instant::now();
        let mut index = SessionIndex::default();
        index.add(Agent::Claude, &claude).map_err(|error| Failure::selection(&error))?;
        index.add(Agent::Codex, &codex).map_err(|error| Failure::selection(&error))?;
        stats.phase("session_index", started);
        Ok(Self { claude, codex, index })
    }

    fn sources(&self) -> [QuerySource<'_>; 2] {
        [
            QuerySource { agent: Agent::Claude, ingested: &self.claude },
            QuerySource { agent: Agent::Codex, ingested: &self.codex },
        ]
    }
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
        self.phases.push((name, started.elapsed()));
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
        .filter_map(|(source, catalog)| selected.contains(&catalog.family).then_some(source))
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
    })
}

fn codex_catalog_sources(discovery: &Discovery) -> Result<Vec<CatalogSource>, Failure> {
    let mut headers = Vec::with_capacity(discovery.sources.len());
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for source in &discovery.sources {
        let thread = codex_thread_from_locator(&source.locator);
        adjacency.entry(thread.clone()).or_default();
        let links = read_codex_session_links(source)?;
        for linked in &links {
            adjacency.entry(thread.clone()).or_default().insert(linked.clone());
            adjacency.entry(linked.clone()).or_default().insert(thread.clone());
        }
        headers.push((thread, source));
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
        .map(|(thread, source)| {
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
            })
        })
        .collect()
}

fn source_paths(source: &DiscoveredSource) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some((path, _)) = source.files.primary() {
        paths.push(path.to_owned());
    }
    if let Some(path) = source.files.twin() {
        paths.push(path.to_owned());
    }
    paths
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

fn read_codex_session_links(source: &DiscoveredSource) -> Result<BTreeSet<String>, Failure> {
    let Some((path, representation)) = source.files.primary() else {
        return Ok(BTreeSet::new());
    };
    let file = File::open(path).map_err(|error| {
        Failure::runtime(format!("cannot open source {}: {error}", path.display()))
    })?;
    let reader: Box<dyn Read> = if representation == Representation::Zstd {
        let decoder = zstd::stream::read::Decoder::new(file).map_err(|error| {
            Failure::runtime(format!("cannot decode source {}: {error}", path.display()))
        })?;
        Box::new(decoder)
    } else {
        Box::new(file)
    };
    let mut reader = BufReader::new(reader).take(MAX_CATALOG_HEADER_BYTES.saturating_add(1));
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).map_err(|error| {
        Failure::runtime(format!("cannot read source {}: {error}", path.display()))
    })?;
    if u64::try_from(line.len()).unwrap_or(u64::MAX) > MAX_CATALOG_HEADER_BYTES {
        return Err(Failure::runtime(format!(
            "cannot inspect Codex session header in {}: first record exceeds {} MiB",
            path.display(),
            MAX_CATALOG_HEADER_BYTES / (1024 * 1024)
        )));
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) else {
        return Ok(BTreeSet::new());
    };
    if value.get("type").and_then(serde_json::Value::as_str) != Some("session_meta") {
        return Ok(BTreeSet::new());
    }
    Ok([
        "/payload/session_id",
        "/payload/parent_thread_id",
        "/payload/forked_from_id",
        "/payload/source/subagent/thread_spawn/parent_thread_id",
    ]
    .into_iter()
    .filter_map(|pointer| value.pointer(pointer).and_then(serde_json::Value::as_str))
    .map(str::to_owned)
    .collect())
}

enum ExplicitDialect {
    Claude(PathBuf),
    Codex(PathBuf),
}

fn classify_explicit_source(source: &Path) -> Result<Vec<ExplicitDialect>, Failure> {
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
        let Some((path, _)) = discovered.files.primary() else { continue };
        // A transcript holding only records no dialect claims, such as a lone title
        // record, says nothing about the root; the other files decide it.
        let Some(candidate) = classify_jsonl(path)? else { continue };
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
        Some(Agent::Pi) => Err(Failure::usage("Pi source input is not supported yet")),
        None => Err(Failure::runtime(format!(
            "source {} contains no supported Claude Code or Codex JSONL records",
            source.display()
        ))),
    }
}

fn classify_jsonl(path: &Path) -> Result<Option<Agent>, Failure> {
    classify_jsonl_with_limit(path, MAX_CATALOG_HEADER_BYTES)
}

/// Classifies a transcript by its first recognized record type, or `None` when none of
/// its first 100 records belongs to a supported dialect.
fn classify_jsonl_with_limit(path: &Path, max_line_bytes: u64) -> Result<Option<Agent>, Failure> {
    let file = File::open(path).map_err(|error| {
        Failure::runtime(format!("cannot open source {}: {error}", path.display()))
    })?;
    let mut reader: Box<dyn BufRead> = if path.to_string_lossy().ends_with(".zst") {
        let decoder = zstd::stream::read::Decoder::new(file).map_err(|error| {
            Failure::runtime(format!("cannot decode source {}: {error}", path.display()))
        })?;
        Box::new(BufReader::new(decoder))
    } else {
        Box::new(BufReader::new(file))
    };
    let mut line = Vec::new();
    for _ in 0..100 {
        line.clear();
        let read = reader
            .by_ref()
            .take(max_line_bytes.saturating_add(1))
            .read_until(b'\n', &mut line)
            .map_err(|error| {
                Failure::runtime(format!("cannot read source {}: {error}", path.display()))
            })?;
        if read == 0 {
            break;
        }
        if u64::try_from(line.len()).unwrap_or(u64::MAX) > max_line_bytes {
            return Err(Failure::runtime(format!(
                "cannot inspect source {}: a classification record exceeds {} MiB",
                path.display(),
                max_line_bytes / (1024 * 1024)
            )));
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
            return Ok(Some(Agent::Codex));
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
            return Ok(Some(Agent::Claude));
        }
    }
    Ok(None)
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
    let current = wants_current
        .then(|| CurrentEnvironment::from_process().detect(&BTreeSet::new()))
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
    let query = SelectionQuery {
        current,
        sessions: args.sessions.clone(),
        all: wants_all,
        scope: Some(scope),
        ..SelectionQuery::default()
    };
    let selection_name = selection_name(&query);
    let corpus = Corpus::discover(args, &query, stats)?;
    let started = Instant::now();
    let selected = corpus.index.select(&query).map_err(|error| Failure::selection(&error))?;
    let all = query.all && query.current.is_none() && query.sessions.is_empty();
    let sources = corpus.sources();
    let metadata = QueryMetadata::new(command.name(), selection_name, scope, &timezone);

    let mut output = match command {
        Command::Report(_) => {
            let groups = args.group_by.iter().copied().map(GroupBy::from).collect();
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
            let document = sessions(&sources, &corpus.index, &selected, all, metadata)
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
        ExplicitDialect, Failure, MAX_JOBS, OutputFormat, ScopeArg, Stats, TerminalContext,
        classify_explicit_source, classify_jsonl_with_limit, decoding_workers,
        derive_agent_thread_id, execute, narrow_discoveries, run, run_with_context,
        stats_requested,
    };
    use clap::Parser;
    use urollup_core::adapters::{AdapterError, claude_project, codex_rollout};
    use urollup_core::ledger::reconcile::ReconcileError;
    use urollup_core::selection::{CurrentSession, Scope, SelectionQuery, SessionIndex};
    use urollup_core::sources::roots;

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
        let exit = run_with_context(args, &mut stdout, &mut stderr, terminals, environment);
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
        let Command::Report(selection) = cli.command else {
            panic!("expected report command");
        };
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
                "--color",
                "--no-progress",
            ] {
                assert!(outcome.stdout.contains(flag), "{command} help lacks {flag}");
            }
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
    fn explicit_artifact_classification_bounds_plain_and_decoded_zstd_lines() {
        let root = tempfile::tempdir().unwrap();
        let line = format!(r#"{{"type":"assistant","padding":"{}"}}\n"#, "x".repeat(128));
        let plain = root.path().join("oversized.jsonl");
        fs::write(&plain, &line).unwrap();
        let compressed = root.path().join("oversized.jsonl.zst");
        fs::write(&compressed, zstd::stream::encode_all(line.as_bytes(), 1).unwrap()).unwrap();

        for path in [plain, compressed] {
            let error = classify_jsonl_with_limit(&path, 32).unwrap_err();
            assert!(error.message.contains("classification record exceeds"));
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
        assert_eq!(classify_jsonl_with_limit(&project.join("aaaa.jsonl"), 1024).unwrap(), None);
        assert_eq!(
            classify_jsonl_with_limit(&project.join("bbbb.jsonl"), 1024).unwrap(),
            Some(Agent::Claude)
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
            ["discovery", "claude_ingest", "codex_ingest", "session_index", "query_render"]
        );
        let [claude, codex] = stats.agents.as_slice() else {
            panic!("expected one row per agent: {:?}", stats.agents);
        };
        assert_eq!((claude.agent, codex.agent), ("claude", "codex"));
        assert!(claude.sources > 0 && claude.requests > 0, "{claude:?}");
        assert!(claude.observations >= u64::try_from(claude.requests).unwrap(), "{claude:?}");
        assert_eq!(codex.sources, 0);
        assert_eq!(codex.observations, 0);
    }

    #[test]
    fn a_capacity_failure_exits_one_and_suggests_narrower_sources() {
        let error = AdapterError::Reconcile(ReconcileError::CapacityExceeded {
            observations: 9_000_001,
            maximum: 9_000_000,
        });
        let failure = Failure::adapter(&error);
        assert_eq!(failure.exit, Exit::Runtime);
        assert_eq!(
            failure.message,
            "9000001 request observations exceed the reconciliation capacity of 9000000 compact \
             rows (2 GiB); pass narrower --source roots with --no-default-sources"
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
