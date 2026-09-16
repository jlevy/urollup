//! Argument parsing, stream handling and exit translation for the `urollup` executable.
//!
//! [`run`] takes stdout and stderr as injected writers, so stream and exit behavior is
//! unit-testable without spawning a process. stdout carries only requested data (help and
//! version text count as requested); every diagnostic goes to stderr.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use urollup_core::adapters::discovery::DiscoveryEnvironment;
use urollup_core::adapters::{AdapterError, Ingested, claude_project, codex_rollout};
use urollup_core::query::{
    GroupBy, QueryMetadata, QuerySource, ResolvedTimeZone, daily, report, sessions,
};
use urollup_core::selection::{
    Agent, CurrentEnvironment, Scope, SelectionError, SelectionQuery, SessionIndex,
};
use urollup_core::sources::roots;

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
    #[command(subcommand)]
    command: Command,
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
pub fn run<I, T>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> Exit
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => return report_parse_outcome(&error, stdout, stderr),
    };
    match execute(&cli.command) {
        Ok(output) => {
            finish_stdout(stdout.write_all(output.as_bytes()).and_then(|()| stdout.flush()), stderr)
        }
        Err(failure) => {
            let _ = writeln!(stderr, "error: {}", failure.message);
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

fn execute(command: &Command) -> Result<String, Failure> {
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
                OutputFormat::Table => crate::render::report(&document),
                OutputFormat::Json => serde_json::to_string_pretty(&document)
                    .map_err(|error| Failure::runtime(error.to_string()))?,
            }
        }
        Command::Daily(_) => {
            let document = daily(&sources, &selected, all, metadata, &timezone)
                .map_err(|error| Failure::runtime(error.to_string()))?;
            match args.format {
                OutputFormat::Table => crate::render::daily(&document),
                OutputFormat::Json => serde_json::to_string_pretty(&document)
                    .map_err(|error| Failure::runtime(error.to_string()))?,
            }
        }
        Command::Sessions(_) => {
            let document = sessions(&sources, &corpus.index, &selected, all, metadata)
                .map_err(|error| Failure::runtime(error.to_string()))?;
            match args.format {
                OutputFormat::Table => crate::render::sessions(&document),
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
) -> Exit {
    let rendered = error.render().to_string();
    if error.use_stderr() {
        // A closed stderr must not change a status that is already decided.
        let _ = stderr.write_all(rendered.as_bytes()).and_then(|()| stderr.flush());
        return Exit::Usage;
    }
    // `--help` and `--version` are requested data, so they go to stdout.
    finish_stdout(stdout.write_all(rendered.as_bytes()).and_then(|()| stdout.flush()), stderr)
}

/// Classify the result of writing and flushing stdout after the required work succeeded.
fn finish_stdout(result: io::Result<()>, stderr: &mut dyn Write) -> Exit {
    match result {
        Ok(()) => Exit::Success,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Exit::Success,
        Err(error) => {
            let _ = writeln!(stderr, "error: failed to write output: {error}");
            Exit::Runtime
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    use std::path::PathBuf;

    use super::{
        Cli, Command, Exit, ExplicitDialect, OutputFormat, ScopeArg, classify_explicit_source, run,
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
