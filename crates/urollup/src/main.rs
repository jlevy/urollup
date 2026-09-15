//! The `urollup` executable: a thin process boundary over `urollup-core`.
//!
//! `main` only sets up the process streams, calls [`cli::run`] with injected writers and
//! translates its [`cli::Exit`] into an [`ExitCode`]; it never calls
//! `std::process::exit`.

mod cli;

use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut diagnostics = stderr.lock();
    let exit = cli::run(std::env::args_os(), &mut out, &mut diagnostics);
    ExitCode::from(exit.code())
}
