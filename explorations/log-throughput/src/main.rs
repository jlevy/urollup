//! Throwaway spike for urollup: measures local Claude Code and Codex log volume,
//! composition and usage-extraction throughput. Prints aggregates only.

mod args;
mod capture;
mod extract;
mod lines;
mod manifest;
mod metrics;
mod run;
mod summarize;

use std::process::ExitCode;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const USAGE: &str = "usage: log-throughput-spike <command> [options]

  manifest  --out FILE [--days 30] [--claude-root DIR] [--codex-root DIR]
  capture   --manifest FILE --out DIR [--threads 10] [--level19] [--digest-hex 64]
  run       --manifest FILE --mode read|value|typed|prefilter|cache|cache-verify
            [--threads 1] [--slice all|window] [--capture DIR] [--rep N]
  summarize --results FILE";

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = raw.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let args = match args::Args::parse(&raw[1..]) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = match command.as_str() {
        "manifest" => manifest::cmd(&args),
        "capture" => capture::cmd(&args),
        "run" => run::cmd(&args),
        "summarize" => summarize::cmd(&args),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
