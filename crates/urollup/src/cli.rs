//! Argument parsing, stream handling and exit translation for the `urollup` executable.
//!
//! [`run_with_context`] takes stdout and stderr as injected writers plus explicit
//! terminal capabilities, so stream and exit behavior is unit-testable without spawning
//! a process. stdout carries only requested data (help and version text count as
//! requested); every diagnostic goes to stderr.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use clap::builder::styling::{AnsiColor, Style as AnsiStyle, Styles};
use clap::{Args, ColorChoice, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use urollup_core::adapters::discovery::DiscoveryEnvironment;
use urollup_core::adapters::{AdapterError, Ingested, claude_project, codex_rollout};
use urollup_core::query::{
    GroupBy, QueryMetadata, QuerySource, ResolvedTimeZone, daily, report, sessions,
};
use urollup_core::selection::{
    Agent, CurrentEnvironment, Scope, SelectionError, SelectionQuery, SessionIndex,
};
use urollup_core::sources::roots;

const STYLE_HEADING: AnsiStyle = AnsiColor::Cyan.on_default().bold();
const STYLE_ERROR: AnsiStyle = AnsiColor::Red.on_default().bold();
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
    let progress = !cli.no_progress && !machine && terminals.stderr_is_terminal;
    if progress {
        write_progress(stderr, "Reading usage logs…");
    }
    let result = execute(&cli.command, stdout_color);
    if progress {
        clear_progress(stderr);
    }
    match result {
        Ok(output) => finish_stdout(
            stdout.write_all(output.as_bytes()).and_then(|()| stdout.flush()),
            stderr,
            stderr_color,
        ),
        Err(failure) => {
            let label = paint("error:", STYLE_ERROR, stderr_color);
            let _ = writeln!(stderr, "{label} {}", failure.message);
            failure.exit
        }
    }
}

struct Corpus {
    claude: Ingested,
    codex: Ingested,
    index: SessionIndex,
}

impl Corpus {
    fn discover(args: &SelectionArgs) -> Result<Self, Failure> {
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
        let claude = claude_project::ingest_roots(&claude_roots, claude_missing_is_error)
            .map_err(|error| Failure::adapter(&error))?;
        let codex = codex_rollout::ingest_roots(&codex_roots, codex_missing_is_error)
            .map_err(|error| Failure::adapter(&error))?;
        let mut index = SessionIndex::default();
        index.add(Agent::Claude, &claude).map_err(|error| Failure::selection(&error))?;
        index.add(Agent::Codex, &codex).map_err(|error| Failure::selection(&error))?;
        Ok(Self { claude, codex, index })
    }

    fn sources(&self) -> [QuerySource<'_>; 2] {
        [
            QuerySource { agent: Agent::Claude, ingested: &self.claude },
            QuerySource { agent: Agent::Codex, ingested: &self.codex },
        ]
    }
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
        let candidate = classify_jsonl(path)?;
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

fn classify_jsonl(path: &Path) -> Result<Agent, Failure> {
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
    let mut line = String::new();
    for _ in 0..100 {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|error| {
            Failure::runtime(format!("cannot read source {}: {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
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
            return Ok(Agent::Codex);
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
            )
        ) {
            return Ok(Agent::Claude);
        }
    }
    Err(Failure::runtime(format!("cannot identify the agent dialect of source {}", path.display())))
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

fn execute(command: &Command, color: bool) -> Result<String, Failure> {
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
    let corpus = Corpus::discover(args)?;
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
    use std::ffi::OsString;
    use std::io::{self, Write};
    use std::path::PathBuf;

    use super::{
        Cli, ColorContext, ColorEnvironment, ColorWhen, Command, Exit, ExplicitDialect,
        OutputFormat, ScopeArg, TerminalContext, classify_explicit_source, run, run_with_context,
    };
    use clap::Parser;

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
