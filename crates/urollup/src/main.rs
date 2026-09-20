//! The `urollup` executable: a thin process boundary over `urollup-core`.
//!
//! `main` only sets up the process streams, calls [`cli::run_with_context`] with injected
//! writers and terminal capabilities, and translates its [`cli::Exit`] into an
//! [`ExitCode`]; it never calls
//! `std::process::exit`.

mod cli;
mod render;

use std::io::{self, IsTerminal};
use std::process::ExitCode;

fn main() -> ExitCode {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let terminals = cli::TerminalContext {
        stdout_is_terminal: stdout.is_terminal(),
        stderr_is_terminal: stderr.is_terminal(),
    };
    let mut out = io::BufWriter::new(stdout.lock());
    let mut diagnostics = stderr.lock();
    let exit = cli::run_with_context(
        std::env::args_os(),
        &mut out,
        &mut diagnostics,
        terminals,
        cli::ColorEnvironment::from_process(),
    );
    ExitCode::from(exit.code())
}
