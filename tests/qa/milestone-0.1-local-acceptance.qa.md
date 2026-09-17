---
title: Milestone 0.1 Local Acceptance QA
description: Reusable manual checks for urollup's terminal behavior, bounded real-log handling, privacy-safe aggregates and local parity workflow.
date: 2026-09-16
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Active; use for milestone 0.1 acceptance and repeat after parser, reconciliation, terminal or local-tool changes
---
# QA Playbook: Milestone 0.1 Local Acceptance

## Purpose

This playbook checks urollup as a maintainer would use it: human-readable reports in a
terminal, JSON in pipelines, bounded real-log parsing under a memory watchdog and the
privacy-safe local aggregate tools.
It complements the hermetic fixture, golden and parity checks in `make check`; it does
not replace them.

The dated
[milestone 0.1 QA report](../../docs/project/qa/qa-report-2026-09-16-milestone-0.1.md)
is one completed record produced from this procedure.

**Estimated duration:** 30–60 minutes after the pinned toolchains and dependencies are
installed. A full-corpus parity run is excluded; whole-history validation follows the
[full-history QA playbook](full-history-rollup.qa.md).

## Safety Boundaries

- Obtain explicit consent before reading default agent logs.
  The local make targets enforce this with `CONSENT_LOCAL_LOGS=1`.
- Never paste raw command output, session identifiers, prompts, local paths or custom
  model names into a QA report.
  Record only the allowlisted aggregate fields named in this playbook.
- Keep captured output under the ignored `target/` tree.
  Do not commit it.
- Run real-log commands through `scripts/run-rss-watchdog.py` with a 1 GiB observed-RSS
  limit. The external sampler kills the supervised process group when a sample exceeds
  the limit and records peak RSS and elapsed milliseconds without retaining its command
  or environment. A process can briefly exceed the limit between 50 ms samples, so this
  is an enforcing watchdog, not an operating-system memory limit.
- urollup no longer limits input size: the temporary 512 MiB input guard and its read
  budgets were removed once compact ingestion read the whole default corpus in under 1
  GiB. Two runs of the earlier unbounded engine caused severe memory pressure, one of
  them started accidentally by shell command substitution, so keep every real-log run
  under the watchdog and validate the whole default corpus with the
  [full-history QA playbook](full-history-rollup.qa.md), not this bounded one.
- Never place backticks inside double-quoted shell arguments, such as a bead description
  that mentions a command.
  The shell executes the quoted command; pass such text through `--file`, stdin or
  single quotes.
- Before a real-log session, reinstall the global developer binary from the current
  checkout so any `urollup` on `PATH`, including one started by accident, runs the
  compact engine rather than an older unbounded build.
- Point `$CLAUDE_CONFIG_DIR` and `$CODEX_HOME` at the consented, bounded QA corpus.
  Confirm that choice without printing either value in logs or reports.
- Set `$UROLLUP_QA_SESSION_SELECTOR` to one consented session in that corpus for the
  descendant-scope check.
  Never print its value in logs or reports.

The watchdog currently supports macOS and Linux and requires `ps` with the standard
`pgid` and `rss` columns.
Exit 97 means it killed the process for observed RSS; exit 98 means the watchdog itself
failed, so neither status is a passing application result.

## Test Status

**Status legend:** ✅ Passed | ❌ Failed | ⏳ In Progress | ⏸️ Blocked | ⬜ Not Run

| Phase | Description | Status |
| --- | --- | --- |
| 1 | Setup, consent gates and automated handoff gate | ✅ Passed |
| 2 | Interactive color, progress, stream and exit behavior | ✅ Passed |
| 3 | Bounded real-log command and aggregate validation | ✅ Passed |
| 4 | Local ccusage parity, review and cleanup | ⏸️ Blocked |

### Test Results: 2026-09-16

- `make check`: ✅ passed after the memory-bound and QA-tool changes.
- Global developer install, version and help smoke test: ✅ passed.
- Consent gates for `make e2e-local` and `make parity-local`: ✅ exited 2 without
  consent.
- Default-corpus safety guard for three `--all` forms: ✅ exited 1 with the safety-limit
  error at about 11 MiB peak RSS.
- Interactive color, progress, plain-stream and failure checks: ✅ passed.
- Bounded Claude Code and Codex corpus under a 1 GiB watchdog: ✅ passed.
- Full roughly 15 GB default corpus and `make parity-local`: ⏸️ blocked on bounded
  streaming or spill.

## Phase 1: Setup and Consent Gates

### Prerequisites

- The pinned Rust toolchains, cargo-deny, uv and npm dependencies described in
  [AGENTS.md](../../AGENTS.md) are installed.
- `$CLAUDE_CONFIG_DIR` and `$CODEX_HOME` identify a consented, bounded QA corpus.
- `$UROLLUP_QA_SESSION_SELECTOR` identifies one consented session in that corpus.
- `ps` provides process-group and RSS columns for the repository watchdog.
- The terminal supports a pseudo-TTY for the interactive checks.

### Steps

1. Build the release binary and run the repository handoff gate:

   ```bash
   cargo build --locked --release --workspace
   make check
   ```

   **Expected:** both commands exit 0. `make check` finishes with every committed gate
   passing.

2. Install the checked-out developer build and smoke-test the global command:

   ```bash
   cargo install --locked --path crates/urollup --force
   urollup --version
   urollup --help
   ```

   **Expected:** installation succeeds, version reports `0.1.0`, and help exits 0. Do
   not record the machine-specific executable path.

3. Confirm that the maintainer-only targets reject accidental access to default logs:

   ```bash
   make e2e-local
   make parity-local
   ```

   **Expected:** each command exits 2 before reading local logs and tells the operator
   to rerun with `CONSENT_LOCAL_LOGS=1`.

4. Prepare ignored capture space and confirm that the two configured roots exist without
   printing their values:

   ```bash
   mkdir -p target/qa
   test -d "$CLAUDE_CONFIG_DIR"
   test -d "$CODEX_HOME"
   test -n "$UROLLUP_QA_SESSION_SELECTOR"
   ```

   **Expected:** all commands exit 0, and `git status --short` shows no new tracked
   artifact.

### Troubleshooting

- If `make check` fails, stop and fix that gate before interpreting manual results.
- If a configured root is absent, correct the environment outside the captured QA log.
  Do not print the private path for diagnosis.
- If either local target starts without consent, mark the phase **Failed**; the consent
  boundary is a release blocker.

### Phase 1 Verification

- [ ] The release build and `make check` exit 0.
- [ ] The globally installed command reports `urollup 0.1.0` and renders help.
- [ ] Both local-only targets reject missing consent with exit 2.
- [ ] The consented roots and selector exist without their values entering captured
  output.

## Phase 2: Terminal Behavior

Run these checks in a real or pseudo-terminal.
Do not capture table rows in the QA report.

### Steps

1. Check automatic human presentation:

   ```bash
   target/release/urollup --help
   target/release/urollup sessions --all --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources
   ```

   **Expected:** help and table headings use terminal color.
   The sessions command shows progress on stderr, clears it, writes the table to stdout
   and exits 0.

2. Check unconditional CLI controls:

   ```bash
   target/release/urollup sessions --all --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources --color never --no-progress
   ```

   **Expected:** the command exits 0 with no color and no progress indicator.

3. Check the standard color-disable environment while retaining interactive progress:

   ```bash
   NO_COLOR=1 target/release/urollup sessions --all --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources
   ```

   **Expected:** stdout has no SGR styling, while stderr still shows and clears the
   progress indicator.

4. Check machine output and redirected streams:

   ```bash
   target/release/urollup sessions --all --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources --format json > target/qa/sessions.json 2> target/qa/sessions.stderr
   test ! -s target/qa/sessions.stderr
   uv --config-file uv.toml run --frozen python -c 'import json, pathlib; p=pathlib.Path("target/qa/sessions.json"); b=p.read_bytes(); json.loads(b); assert not {x for x in b if x < 32 and x not in b"\t\n\r "}'
   ```

   **Expected:** the JSON is valid, stderr is empty and machine stdout contains no ANSI
   escape or other disallowed control byte.

5. Check failure status and progress cleanup with a synthetic selector:

   ```bash
   target/release/urollup report --session urollup-qa-selector-that-does-not-exist --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources
   ```

   **Expected:** the command exits 1, the progress line is cleared before the error and
   the terminal prompt begins on a clean line.

### Troubleshooting

- Confirm that stdout and stderr are attached to a terminal before diagnosing missing
  automatic color or progress.
- JSON is machine-readable even in a terminal: it must stay plain and must suppress
  progress regardless of `--color always`.
- `NO_COLOR` controls color only.
  Use `--no-progress` when interactive progress must also be disabled.
- A progress line that remains after success or error is a failed cleanup check even if
  the command returned the expected status.

### Phase 2 Verification

- [ ] Automatic color and the scan indicator appear in an interactive terminal.
- [ ] `--color never --no-progress` suppresses both unconditionally.
- [ ] `NO_COLOR=1` suppresses color without suppressing interactive progress.
- [ ] Redirected JSON is valid and contains no ANSI or disallowed control byte.
- [ ] The negative selector exits 1 and leaves a clean terminal line.

## Phase 3: Bounded Real-Log Validation

### Steps

1. Run each command against only the two configured roots, using the repository’s 1 GiB
   observed-RSS watchdog:

   ```bash
   uv --config-file uv.toml run --frozen python scripts/run-rss-watchdog.py --limit-mib 1024 --report target/qa/sessions-rss.json -- target/release/urollup sessions --all --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources --format json --no-progress > target/qa/sessions.json
   uv --config-file uv.toml run --frozen python scripts/run-rss-watchdog.py --limit-mib 1024 --report target/qa/daily-rss.json -- target/release/urollup daily --all --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources --format json --no-progress > target/qa/daily.json
   uv --config-file uv.toml run --frozen python scripts/run-rss-watchdog.py --limit-mib 1024 --report target/qa/grouped-rss.json -- target/release/urollup report --all --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources --group-by model,project --format json --no-progress > target/qa/grouped.json
   uv --config-file uv.toml run --frozen python scripts/run-rss-watchdog.py --limit-mib 1024 --report target/qa/descendants-rss.json -- target/release/urollup report --session "$UROLLUP_QA_SESSION_SELECTOR" --scope descendants --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources --format json --no-progress > target/qa/descendants.json
   ```

   **Expected:** every command exits 0, each urollup document is valid JSON and each
   watchdog report has `killed_for_rss: false`, `exit_code: 0`, a peak at or below
   1,048,576 KiB and no `watchdog_error`. Record only command class and the fixed metric
   fields; the watchdog never stores the command or environment.

2. From inside a recognized agent session, verify exact current-session discovery:

   ```bash
   uv --config-file uv.toml run --frozen python scripts/run-rss-watchdog.py --limit-mib 1024 --report target/qa/current-rss.json -- target/release/urollup report --current --source "$CLAUDE_CONFIG_DIR" --source "$CODEX_HOME" --no-default-sources --format json --no-progress > target/qa/current.json
   ```

   **Expected:** the command exits 0 and emits valid JSON. Do not record the selected
   identifier.

3. Run the allowlisted local aggregate only after explicit consent:

   ```bash
   make e2e-local CONSENT_LOCAL_LOGS=1
   ```

   **Expected:** the command exits 0 and writes
   `target/acceptance/local-aggregate.json`.

4. Review only these aggregate fields: session-row and daily-row counts; owned,
   ambiguous and unknown request counts; token totals by fixed token class; coverage
   counters and completeness; request-size count, percentiles and maximum; diagnostic
   count. Never copy rows, identifiers, paths, prompts or custom model values.

### Troubleshooting

- A watchdog kill is **Blocked**, not a successful command and not permission to retry
  without the guard.
- `coverage.complete: false` is not a parser crash.
  Record the associated fixed coverage counters and investigate whether the partial
  coverage is expected.
- Diagnostics require investigation, but their raw text may contain private context.
  Record only the count until the content is sanitized.
- If the full default corpus is needed, use the full-history QA playbook rather than
  widening this bounded run.

### Phase 3 Verification

- [ ] All five command forms exit 0 and emit valid JSON from the release binary.
- [ ] Every watchdog report records no kill, no watchdog error and peak RSS at or below
  1 GiB.
- [ ] The allowlisted aggregate contains no private field and records partial coverage
  explicitly when present.
- [ ] No selector, path, prompt, raw record or custom model name enters the dated
  report.

## Phase 4: Local Parity, Review and Cleanup

### Steps

1. Run local ccusage comparison only after the bounded urollup checks pass:

   ```bash
   make parity-local CONSENT_LOCAL_LOGS=1
   ```

   **Expected:** the command exits 0 and writes `target/parity/local.json` with only the
   allowlisted fields documented in [the parity harness](../parity/README.md).

2. Review aggregate deltas and threshold flags.
   Explain every nonzero unexplained residual or threshold exceedance in a bead before
   accepting the phase.

3. Confirm the manual run did not change tracked files:

   ```bash
   git status --short
   ```

   **Expected:** only pre-existing worktree changes are present.
   Captures remain under ignored `target/` directories and are never added to Git.

4. Update this playbook’s phase table and dated Test Results section, then record the
   privacy-safe measurements and disposition in the dated report.
   Mark any skipped full-corpus work **Blocked** or **Not Run**, never **Passed**.

### Troubleshooting

- The local parity helper performs two uncached whole-history urollup scans.
  On a large history this can be slow and memory-intensive; do not bypass the watchdog
  to finish it.
- A tool failure may be reported without its raw diagnostic by design.
  Reproduce it locally, sanitize the finding and record only privacy-safe evidence.
- If aggregate deltas are unexplained, preserve the ignored output for local inspection
  and create a bead without copying private values.

### Phase 4 Verification

- [ ] Local parity passes with every nonzero residual explained, or remains explicitly
  blocked.
- [ ] The playbook status, dated test results and privacy-safe QA report agree.
- [ ] Manual captures remain ignored and no unexpected tracked file changed.

## Success Criteria

- [ ] `make check` passes, and both maintainer-only targets reject missing consent.
- [ ] Interactive table output uses automatic color and stderr-only progress; CLI and
  environment controls disable them as specified; success and error paths clean up.
- [ ] Redirected and JSON output contain no ANSI or other control sequences, and JSON
  never emits progress.
- [ ] Every bounded real-log command exits 0 under the 1 GiB watchdog with valid JSON.
- [ ] The local aggregate contains only allowlisted fields, and any partial coverage is
  recorded explicitly.
- [ ] Local parity either passes with explained deltas or remains clearly marked as
  blocked.
- [ ] No private identifiers, paths, prompts, raw records or custom model names enter
  the QA report or Git history.

## Related Documents

- [urollup Design](../../docs/urollup-design.md)
- [Milestone 0.1 Implementation Plan](../../docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md)
- [Golden Testing Audit](../../docs/project/research/research-2026-09-15-golden-testing-audit.md)
- [Parity Harness](../parity/README.md)
- [2026-09-16 Milestone 0.1 QA Report](../../docs/project/qa/qa-report-2026-09-16-milestone-0.1.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
