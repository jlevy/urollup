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
        assert!(String::from_utf8_lossy(&output.stdout).contains("--format"), "{command}");
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
