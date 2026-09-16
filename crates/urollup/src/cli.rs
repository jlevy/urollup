//! Argument parsing, stream handling and exit translation for the `urollup` executable.
//!
//! [`run`] takes stdout and stderr as injected writers, so stream and exit behavior is
//! unit-testable without spawning a process. stdout carries only requested data (help and
//! version text count as requested); every diagnostic goes to stderr.

use std::ffi::OsString;
use std::io::{self, Write};

use clap::{Parser, Subcommand};

/// The process exit classes this scaffold can produce.
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
    /// Invalid invocation or request, including a command that is not implemented yet.
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

#[derive(Clone, Copy, Debug, Subcommand)]
enum Command {
    /// Session report: totals, breakdowns, sizes, tools and limitations
    Report,
    /// Calendar rollup by day
    Daily,
    /// One row per session
    Sessions,
}

impl Command {
    fn name(self) -> &'static str {
        match self {
            Self::Report => "report",
            Self::Daily => "daily",
            Self::Sessions => "sessions",
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
    not_implemented(cli.command, stderr)
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

/// The milestone 0.1 commands exist as names only until their reports are implemented.
fn not_implemented(command: Command, stderr: &mut dyn Write) -> Exit {
    let _ = writeln!(
        stderr,
        "error: `urollup {}` is not implemented yet; this build is the repository scaffold",
        command.name()
    );
    Exit::Usage
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use super::{Exit, run};

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
    fn stub_commands_exit_two_with_a_diagnostic_and_no_data() {
        for command in ["report", "daily", "sessions"] {
            let outcome = invoke(&[command]);
            assert_eq!(outcome.exit, Exit::Usage, "{command}");
            assert_eq!(outcome.stdout, "", "{command}");
            assert_eq!(
                outcome.stderr,
                format!(
                    "error: `urollup {command}` is not implemented yet; \
                     this build is the repository scaffold\n"
                ),
            );
        }
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
        assert_eq!(run(["urollup", "report"], &mut stdout, &mut stderr), Exit::Usage);
        assert_eq!(run(["urollup"], &mut stdout, &mut stderr), Exit::Usage);
    }
}
