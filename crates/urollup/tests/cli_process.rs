//! Process-level exit and stream contract for the built `urollup` binary.
//!
//! The unit tests in `src/cli.rs` cover stream and exit classification through injected
//! writers; these tests prove `main` carries that classification to the real process
//! status. They run without Node, so every `cargo test` platform checks them.

use std::process::{Command, Output};

fn urollup(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_urollup"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("the urollup binary runs")
}

fn urollup_without_color_environment(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_urollup"))
        .args(args)
        .env_remove("NO_COLOR")
        .env_remove("FORCE_COLOR")
        .output()
        .expect("the urollup binary runs")
}

#[test]
fn version_exits_zero_on_stdout() {
    let output = urollup(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("urollup {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn report_command_help_exits_zero_on_stdout() {
    for command in ["report", "daily", "sessions"] {
        let output = urollup(&[command, "--help"]);
        assert_eq!(output.status.code(), Some(0), "{command}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("--format"), "{command}");
        assert!(stdout.contains("--max-ram"), "{command}");
        assert!(stdout.contains("--max-rows"), "{command}");
        assert!(stdout.contains("stricter"), "{command}");
        assert!(output.stderr.is_empty(), "{command}");
    }
}

#[test]
fn explicit_color_controls_real_process_help() {
    let colored = urollup_without_color_environment(&["--color", "always", "--help"]);
    assert!(colored.status.success());
    assert!(colored.stdout.windows(2).any(|bytes| bytes == b"\x1b["));
    assert!(colored.stderr.is_empty());

    let plain = urollup_without_color_environment(&["--color", "never", "--help"]);
    assert!(plain.status.success());
    assert!(!plain.stdout.windows(2).any(|bytes| bytes == b"\x1b["));
    assert!(plain.stderr.is_empty());
}

#[test]
fn usage_errors_exit_two() {
    assert_eq!(urollup(&["--no-such-flag"]).status.code(), Some(2));
    let bare = urollup(&[]);
    assert_eq!(bare.status.code(), Some(2));
    assert!(bare.stdout.is_empty());
    // Run through a full path, as here, the usage line still names `urollup` rather than
    // `urollup.exe` or the path, so CLI goldens hold on every platform.
    let stderr = String::from_utf8_lossy(&bare.stderr);
    assert!(stderr.contains("Usage: urollup [OPTIONS] <COMMAND>"), "{stderr}");
}

/// A JSON report over a multi-source fixture with only `variable` of the urollup tuning
/// variables set, to `value`.
fn fixture_report(variable: &str, value: &str) -> Output {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../urollup-core/tests/fixtures/claude-project/workflow-subagents"
    );
    Command::new(env!("CARGO_BIN_EXE_urollup"))
        .args(["report", "--source", source, "--no-default-sources", "--format", "json"])
        .env("NO_COLOR", "1")
        .env_remove("UROLLUP_JOBS")
        .env_remove("UROLLUP_STATS")
        .env_remove("UROLLUP_MAX_RAM")
        .env(variable, value)
        .output()
        .expect("the urollup binary runs")
}

#[test]
fn jobs_variable_sets_workers_without_changing_output() {
    let one = fixture_report("UROLLUP_JOBS", "1");
    assert_eq!(one.status.code(), Some(0), "{}", String::from_utf8_lossy(&one.stderr));
    assert!(!one.stdout.is_empty());
    for jobs in ["2", "8"] {
        let parallel = fixture_report("UROLLUP_JOBS", jobs);
        assert_eq!(parallel.status.code(), Some(0), "UROLLUP_JOBS={jobs}");
        assert_eq!(parallel.stdout, one.stdout, "UROLLUP_JOBS={jobs}");
    }
}

#[test]
fn invalid_jobs_variable_is_a_usage_error() {
    for jobs in ["0", "many"] {
        let output = fixture_report("UROLLUP_JOBS", jobs);
        assert_eq!(output.status.code(), Some(2), "UROLLUP_JOBS={jobs}");
        assert!(output.stdout.is_empty(), "UROLLUP_JOBS={jobs}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("UROLLUP_JOBS must be a whole number"), "{stderr}");
    }
}

#[test]
fn stats_variable_writes_privacy_safe_stats_to_stderr_only() {
    let plain = fixture_report("UROLLUP_STATS", "0");
    assert_eq!(plain.status.code(), Some(0), "{}", String::from_utf8_lossy(&plain.stderr));
    assert!(plain.stderr.is_empty());

    let with_stats = fixture_report("UROLLUP_STATS", "1");
    assert_eq!(with_stats.status.code(), Some(0));
    assert_eq!(with_stats.stdout, plain.stdout, "stats never change stdout");
    let stderr = String::from_utf8(with_stats.stderr).expect("stats are UTF-8");
    let lines: Vec<&str> = stderr.lines().collect();
    assert!(lines.iter().all(|line| line.starts_with("stats: ")), "{stderr}");
    for expected in [
        "stats: workers=",
        "stats: phase=discovery seconds=",
        "stats: phase=claude_ingest seconds=",
        "stats: phase=codex_ingest seconds=",
        "stats: phase=session_index seconds=",
        "stats: phase=query_render seconds=",
        "stats: agent=claude sources=",
        "stats: agent=codex sources=0 observations=0 requests=0 ",
        "stats: total seconds=",
    ] {
        assert!(lines.iter().any(|line| line.starts_with(expected)), "no {expected:?}: {stderr}");
    }
    // Only names, numbers and `key=value` pairs: no path, analytical ID or model name.
    assert!(stderr.chars().all(|c| c.is_ascii_alphanumeric() || " :=._\n".contains(c)), "{stderr}");
    for private in ["workflow", "thr-", "src-", "req-", "claude-", "fixtures"] {
        assert!(!stderr.contains(private), "{private}: {stderr}");
    }
}

#[test]
fn tiny_max_rows_exits_one_with_a_capacity_diagnostic() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../urollup-core/tests/fixtures/claude-project/workflow-subagents"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_urollup"))
        .args([
            "report",
            "--source",
            source,
            "--no-default-sources",
            "--format",
            "json",
            "--max-rows",
            "1",
        ])
        .env("NO_COLOR", "1")
        .env_remove("UROLLUP_JOBS")
        .env_remove("UROLLUP_STATS")
        .env_remove("UROLLUP_MAX_RAM")
        .output()
        .expect("the urollup binary runs");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exceed the reconciliation capacity"), "{stderr}");
    assert!(stderr.contains("pass narrower --source roots with --no-default-sources"), "{stderr}");
}

#[test]
fn invalid_max_ram_variable_is_a_usage_error() {
    let output = fixture_report("UROLLUP_MAX_RAM", "many");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("UROLLUP_MAX_RAM is invalid"), "{stderr}");
}

#[test]
fn invalid_stats_variable_is_a_usage_error() {
    for value in ["2", "yes"] {
        let output = fixture_report("UROLLUP_STATS", value);
        assert_eq!(output.status.code(), Some(2), "UROLLUP_STATS={value}");
        assert!(output.stdout.is_empty(), "UROLLUP_STATS={value}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.starts_with("error: UROLLUP_STATS must be 1"), "{stderr}");
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
fn percent_budget_works_without_helper_programs_and_keeps_home_untouched() {
    let home = tempfile::tempdir().expect("isolated HOME");
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../urollup-core/tests/fixtures/claude-project/workflow-subagents"
    );
    // An explicit percent requires real RAM discovery: falling back to 2 GiB would
    // fail this command. No shell/helper executable is available through PATH.
    let output = Command::new(env!("CARGO_BIN_EXE_urollup"))
        .args([
            "report",
            "--source",
            source,
            "--no-default-sources",
            "--format",
            "json",
            "--max-ram",
            "25%",
        ])
        .env("PATH", home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("APPDATA", home.path())
        .env("LOCALAPPDATA", home.path())
        .env("NO_COLOR", "1")
        .env_remove("UROLLUP_JOBS")
        .env_remove("UROLLUP_STATS")
        .env_remove("UROLLUP_MAX_RAM")
        .output()
        .expect("the urollup binary runs");
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!output.stdout.is_empty());
    assert_eq!(std::fs::read_dir(home.path()).expect("HOME remains readable").count(), 0);
}
