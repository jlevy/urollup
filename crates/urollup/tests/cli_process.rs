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
    for (ram, observations, maximum, label) in
        [(None, 2, 1, "1 rows"), (Some("1G"), 2, 1, "1 rows"), (Some("1B"), 1, 0, "1 bytes")]
    {
        let mut command = Command::new(env!("CARGO_BIN_EXE_urollup"));
        command.args([
            "report",
            "--source",
            source,
            "--no-default-sources",
            "--format",
            "json",
            "--max-rows",
            "1",
        ]);
        if let Some(ram) = ram {
            command.args(["--max-ram", ram]);
        }
        let output = command
            .env("NO_COLOR", "1")
            .env_remove("UROLLUP_JOBS")
            .env_remove("UROLLUP_STATS")
            .env_remove("UROLLUP_MAX_RAM")
            .output()
            .expect("the urollup binary runs");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "error: {observations} request observations exceed the reconciliation capacity of {maximum} compact rows ({label}); pass narrower --source roots with --no-default-sources\n"
            ),
        );
    }
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

/// Copies a fixture case, storing each `.jsonl` file as `.jsonl<suffix>` encoded by
/// `encode` and every other file, such as a subagent's `.meta.json`, unchanged.
fn copy_compressed(
    case: &std::path::Path,
    destination: &std::path::Path,
    suffix: &str,
    encode: fn(&[u8]) -> Vec<u8>,
) {
    let mut pending = vec![case.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("fixture directory is readable") {
            let path = entry.expect("fixture entry is readable").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let mut target =
                destination.join(path.strip_prefix(case).expect("file is under its case"));
            std::fs::create_dir_all(target.parent().expect("target has a parent"))
                .expect("target directory is writable");
            let mut contents = std::fs::read(&path).expect("fixture file is readable");
            if path.extension().is_some_and(|extension| extension == "jsonl") {
                let name = format!(
                    "{}{suffix}",
                    target.file_name().expect("fixture file has a name").to_string_lossy()
                );
                target.set_file_name(name);
                contents = encode(&contents);
            }
            std::fs::write(target, contents).expect("copy is writable");
        }
    }
}

fn gzip(contents: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(contents).expect("gzip encodes in memory");
    encoder.finish().expect("gzip finishes in memory")
}

fn zstd(contents: &[u8]) -> Vec<u8> {
    zstd::stream::encode_all(contents, 3).expect("zstd encodes in memory")
}

fn json_view(args: &[&str], source: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_urollup"))
        .args(args)
        .arg("--source")
        .arg(source)
        .args(["--no-default-sources", "--format", "json", "--timezone", "UTC"])
        .env("NO_COLOR", "1")
        .env_remove("UROLLUP_STATS")
        .output()
        .expect("the urollup binary runs")
}

#[test]
fn gzip_and_zstd_logs_report_exactly_like_plain_logs() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
    for case in ["claude-project/workflow-subagents", "codex-rollout/legacy-subagent-prefix"] {
        let plain = fixtures.join(case);
        for (suffix, encode) in [(".gz", gzip as fn(&[u8]) -> Vec<u8>), (".zst", zstd)] {
            let copy = tempfile::tempdir().expect("temporary directory is created");
            copy_compressed(&plain, copy.path(), suffix, encode);
            for view in [&["report", "--all"][..], &["daily", "--all"], &["sessions", "--all"]] {
                let expected = json_view(view, &plain);
                assert_eq!(expected.status.code(), Some(0), "{case} {view:?}");
                let actual = json_view(view, copy.path());
                assert_eq!(
                    actual.status.code(),
                    Some(0),
                    "{case}{suffix} {view:?}: {}",
                    String::from_utf8_lossy(&actual.stderr)
                );
                assert_eq!(
                    String::from_utf8_lossy(&actual.stdout),
                    String::from_utf8_lossy(&expected.stdout),
                    "{case}{suffix} {view:?}"
                );
            }
        }
    }
}

#[test]
fn a_compressed_transcript_path_selects_its_session() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
    let plain = fixtures.join("claude-project/workflow-subagents");
    let copy = tempfile::tempdir().expect("temporary directory is created");
    copy_compressed(&plain, copy.path(), ".gz", gzip);
    let transcript = "projects/-Users-example-project/00000000-0000-4000-8000-001100000001.jsonl";

    let by_plain_path = json_view(
        &[
            "report",
            "--scope",
            "descendants",
            "--session",
            &plain.join(transcript).to_string_lossy(),
        ],
        &plain,
    );
    let by_gzip_path = json_view(
        &[
            "report",
            "--scope",
            "descendants",
            "--session",
            &copy.path().join(format!("{transcript}.gz")).to_string_lossy(),
        ],
        copy.path(),
    );
    assert_eq!(
        by_plain_path.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&by_plain_path.stderr)
    );
    assert_eq!(
        by_gzip_path.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&by_gzip_path.stderr)
    );
    assert_eq!(by_gzip_path.stdout, by_plain_path.stdout);

    // One compressed file named directly is classified and read like its directory.
    let single = json_view(&["report", "--all"], &copy.path().join(format!("{transcript}.gz")));
    assert_eq!(single.status.code(), Some(0), "{}", String::from_utf8_lossy(&single.stderr));
}

#[test]
fn a_damaged_compressed_rollout_is_reported_without_stopping_any_report() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
    let clean = fixtures.join("codex-rollout/token-usage-records");
    let copy = tempfile::tempdir().expect("temporary directory is created");
    copy_compressed(&clean, copy.path(), "", <[u8]>::to_vec);
    // An interrupted compressor leaves an empty `.jsonl.gz` for another thread.
    std::fs::write(
        copy.path().join(
            "sessions/2026/09/04/\
             rollout-2026-09-04T10-00-00-019f0000-0000-7000-8000-00ee00000001.jsonl.gz",
        ),
        b"",
    )
    .expect("damaged rollout is writable");
    let parse = |output: &Output| -> serde_json::Value {
        serde_json::from_slice(&output.stdout).expect("the report is JSON")
    };

    for (view, under) in [
        (&["report", "--all"][..], ""),
        (&["report", "--session", "019f0000-0000-7000-8000-000500000001"], ""),
        // A directory without the standard layout is classified by reading its files.
        (&["report", "--all"], "sessions"),
    ] {
        let damaged = json_view(view, &copy.path().join(under));
        assert_eq!(
            damaged.status.code(),
            Some(0),
            "{view:?} {under}: {}",
            String::from_utf8_lossy(&damaged.stderr)
        );
        let report = parse(&damaged);
        assert_eq!(report["coverage"]["complete"], false, "{view:?} {under}");
        assert_eq!(
            report["diagnostics"],
            serde_json::json!([{
                "code": "source-incomplete",
                "count": 1,
                "detail": "a Codex rollout could not be read completely: incomplete-compressed-frame",
            }]),
            "{view:?} {under}"
        );
        let expected = parse(&json_view(view, &clean.join(under)));
        assert_eq!(report["totals"], expected["totals"], "{view:?} {under}: readable totals");
    }
}

#[test]
fn an_unverifiable_codex_fork_boundary_does_not_stop_the_report() {
    let claude = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../urollup-core/tests/fixtures/claude-project/workflow-subagents"
    );
    let codex = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../urollup-core/tests/fixtures/codex-rollout/unverified-fork-boundary"
    );
    let run = |command: &str| {
        let output = Command::new(env!("CARGO_BIN_EXE_urollup"))
            .args([command, "--source", claude, "--source", codex, "--no-default-sources"])
            .args(["--format", "json", "--timezone", "UTC"])
            .env("NO_COLOR", "1")
            .env_remove("UROLLUP_JOBS")
            .env_remove("UROLLUP_STATS")
            .env_remove("UROLLUP_MAX_RAM")
            .output()
            .expect("the urollup binary runs");
        assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("JSON output")
    };

    let sessions = run("sessions");
    let agents: Vec<&str> = sessions["rows"]
        .as_array()
        .expect("session rows")
        .iter()
        .filter(|row| row["tokens"]["total"].as_u64().is_some_and(|total| total > 0))
        .filter_map(|row| row["agent"].as_str())
        .collect();
    assert!(agents.contains(&"claude"), "Claude still reports: {agents:?}");
    assert!(agents.contains(&"codex"), "the healthy Codex sessions still report: {agents:?}");

    let report = run("report");
    assert_eq!(report["coverage"]["complete"], serde_json::Value::Bool(false));
    let diagnostic_codes: Vec<&str> = report["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect();
    assert!(
        diagnostic_codes.contains(&"codex-history-boundary-unverified"),
        "{diagnostic_codes:?}"
    );
}

#[test]
fn an_inconsistent_codex_record_is_a_gap_beside_a_claude_session() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../urollup-core/tests/fixtures");
    let claude = fixtures.join("claude-project/missing-request-id");
    let codex = tempfile::tempdir().expect("temporary directory is created");
    copy_compressed(
        &fixtures.join("codex-rollout/cache-write-input"),
        codex.path(),
        "",
        <[u8]>::to_vec,
    );
    let thread = "019f0000-0000-7000-8000-002200000001";
    let lines = [
        serde_json::json!({"timestamp":"2026-09-21T15:00:00.000Z","type":"session_meta","payload":{"id":thread,"cli_version":"0.154.0","source":"cli","model_provider":"openai"}}),
        // Below its cache reads plus writes, and below its cache reads alone.
        serde_json::json!({"timestamp":"2026-09-21T15:00:10.000Z","type":"token_usage_record","payload":{"thread_id":thread,"response_id":"writes","usage":{"input_tokens":100,"cached_input_tokens":60,"cache_write_input_tokens":50,"output_tokens":5}}}),
        serde_json::json!({"timestamp":"2026-09-21T15:00:20.000Z","type":"token_usage_record","payload":{"thread_id":thread,"response_id":"reads","usage":{"input_tokens":10,"cached_input_tokens":11,"cache_write_input_tokens":0,"output_tokens":1}}}),
        serde_json::json!({"timestamp":"2026-09-21T15:00:30.000Z","type":"token_usage_record","payload":{"thread_id":thread,"response_id":"valid","usage":{"input_tokens":40,"cached_input_tokens":10,"cache_write_input_tokens":0,"output_tokens":4}}}),
    ];
    let bad = codex.path().join("sessions/2026/09/21");
    std::fs::create_dir_all(&bad).expect("rollout directory is created");
    let text = lines.iter().map(serde_json::Value::to_string).collect::<Vec<_>>().join("\n") + "\n";
    std::fs::write(bad.join(format!("rollout-2026-09-21T08-00-00-{thread}.jsonl")), text)
        .expect("rollout is writable");
    let run = |command: &str| {
        let output = Command::new(env!("CARGO_BIN_EXE_urollup"))
            .args([command, "--no-default-sources", "--source"])
            .arg(&claude)
            .arg("--source")
            .arg(codex.path())
            .args(["--format", "json", "--timezone", "UTC"])
            .env("NO_COLOR", "1")
            .env_remove("UROLLUP_JOBS")
            .env_remove("UROLLUP_STATS")
            .env_remove("UROLLUP_MAX_RAM")
            .output()
            .expect("the urollup binary runs");
        assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("complete JSON output")
    };

    let sessions = run("sessions");
    let totals: std::collections::BTreeMap<String, u64> = sessions["rows"]
        .as_array()
        .expect("session rows")
        .iter()
        .filter_map(|row| {
            Some((row["session"].as_str()?.to_owned(), row["tokens"]["total"].as_u64()?))
        })
        .collect();
    let agents: std::collections::BTreeSet<&str> = sessions["rows"]
        .as_array()
        .expect("session rows")
        .iter()
        .filter_map(|row| row["agent"].as_str())
        .collect();
    assert_eq!(agents, ["claude", "codex"].into(), "both agents report");
    assert_eq!(totals.get(thread), Some(&44), "only the consistent record counts: {totals:?}");
    assert_eq!(totals.get("019f0000-0000-7000-8000-002000000001"), Some(&2_810));

    let report = run("report");
    assert_eq!(report["coverage"]["complete"], serde_json::Value::Bool(false));
    assert_eq!(
        report["diagnostics"],
        serde_json::json!([{
            "code": "codex-inclusive-input-underflow",
            "count": 2,
            "detail": format!(
                "Codex input_tokens 100 is below its 60 cached and 50 cache-write tokens at \
                 2026/09/21/rollout-2026-09-21T08-00-00-{thread}.jsonl:2, so that usage is \
                 excluded as a coverage gap; 2 records or counter steps of this rollout are \
                 excluded"
            ),
        }])
    );
}
