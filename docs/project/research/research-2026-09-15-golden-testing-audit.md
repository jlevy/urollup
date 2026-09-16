---
title: Golden Testing Audit and End-to-End Strategy
description: An audit of urollup's tryscript golden harness against the golden-testing guidelines and tryscript 0.2.1, the gaps it found and how they were closed, and the two-layer end-to-end strategy for fixture cases, including a safe path to real-log realism.
date: 2026-09-15
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete; the fixed gaps are in the milestone 0.1 harness, and the remaining items are beads or maintainer decisions named below
---
# Research: Golden Testing Audit and End-to-End Strategy

## Overview

The milestone 0.1 scaffold shipped one tryscript session,
`tests/golden/cli-surface.tryscript.md`, with a runner, an invocation check and a
portability check adapted from fdu.
This audit compares that harness with `tbd guidelines golden-testing-guidelines`,
`general-testing-rules` and `rust-testing-rules`, and with the documented behavior of
the pinned **tryscript 0.2.1** (`node_modules/.bin/tryscript docs` and `readme`, from
the package pinned in `package.json`), before the reports that goldens will record are
built.

It asks one question per rule: what could pass while testing nothing, or pass only on
the machine that recorded it?
Everything the audit found is either fixed in the harness now or named below as a bead
or a maintainer decision.
The resulting strategy is in the plan’s
[golden and end-to-end result checks](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#golden-and-end-to-end-result-checks),
and the operating instructions are in
[tests/golden/README.md](../../../tests/golden/README.md).

## Findings

Severity is what the gap would have cost: **silent pass** means a run could have been
green while checking nothing, **private data** means a run could have read real logs,
and **one machine** means committed data would have reproduced only where it was
recorded.

| # | Gap | Severity | Resolution |
| --- | --- | --- | --- |
| 1 | Sessions inherited the invoking environment, which inside a coding agent names that agent’s live session (`CLAUDE_CODE_SESSION_ID`, `CODEX_THREAD_ID`, `PI_SESSION_FILE`) | Private data | `scripts/golden-env.mjs` builds an allowlisted environment; every agent, override and credential variable is dropped |
| 2 | `HOME` was the maintainer’s, so a report command would have read `~/.claude` and `~/.codex` | Private data | `HOME`, the Windows profile variables and the XDG directories point into a fresh `urollup-golden-*` root |
| 3 | Nothing would have noticed a session reading default roots rather than its own | Silent pass | Canary logs in every default discovery root under that HOME, named `urollup-home-canary` in path, model and working directory; the corpus lint refuses the token in committed data, and the results checker fails on it in output |
| 4 | Nothing would have noticed a run writing into `HOME` (a capture store lands in milestone 0.3) | Silent pass | The runners snapshot the hermetic HOME and fail on any file created, changed or removed; `UROLLUP_CAPTURE_DIR` points outside it |
| 5 | A `<!-- skip -->` block is reported by tryscript as passed, and `<!-- only -->` drops every other block | Silent pass | The lint refuses both annotations, and the runner requires the pass count to equal the console blocks in the selected sessions |
| 6 | Corpus discovery read only the top level of `tests/golden`, so a session in a subdirectory would never have run | Silent pass | Recursive discovery for the runner, the lint and the portability check |
| 7 | Committed unknown wildcards (`[??]`, `???`) match anything; tryscript only warns | Silent pass | The lint refuses them, and the new-case workflow expands them before commit |
| 8 | `tryscript run --update` rewrote the corpus over whatever else was uncommitted, so the diff to review was not what the update wrote | Review quality | `run-golden.mjs --update` refuses unstaged golden changes (`--allow-dirty` overrides) and prints `git diff --stat` afterwards |
| 9 | A session could use shell syntax that `/bin/sh` and `cmd.exe` read differently, so Windows CI would fail on the session rather than on the behavior | One machine | The lint requires bare `urollup` commands; paths reach them through `sandbox:` copies and front matter |
| 10 | A reading command without `--timezone` buckets by the machine zone, and Windows ignores `TZ` | One machine | The lint requires `--timezone` on any reading command expected to succeed; the harness also fixes `TZ`, `LANG`, `LC_ALL` and `NO_COLOR` |
| 11 | No named home for the values that legitimately vary, so an update would have expanded them into literals | One machine | `tests/golden/tryscript.config.mjs` defines `[GOLDEN_HOME]`, `[GOLDEN_FIXTURES]`, `[NOW]`, `[ELAPSED]` and `[UROLLUP_VERSION]`, and refuses a tryscript run outside the harness |
| 12 | No structured layer beside the transcripts: a wrong total and a formatting change would have failed the same way, and fixtures had no committed truth to check | Coverage | `scripts/check-e2e-results.mjs` compares reconciled results with each case’s `expected.json` and prints the naive-sum overcount |
| 13 | No fixture-to-golden mapping, so a new dialect case could land with no golden and no result check | Coverage | One case directory maps to one session at `tests/golden/e2e/<dialect>/<case>.tryscript.md`; `scripts/new-e2e-golden.mjs` scaffolds it, a case without a golden fails once `report` exists, and a golden without a case always fails |
| 14 | Commands exit 2 as scaffold stubs, and a skipped check tends to stay skipped | Silent pass | Pending entries name a bead and fail the run once their blocker is gone, for both the commands and the fixture corpus |
| 15 | Output ordering was never checked, though deterministic output is a milestone 0.1 requirement | Coverage | Each command runs twice per case and must print identical bytes |
| 16 | The harness scripts hold gate decisions but had no tests of their own | Coverage | `node --test` units for every script, run by `make golden-lint` and `make e2e-results`, plus gate probes for a skipped block, an ambient environment reference and a stale pending entry |

Two findings are not fully closed:

- **Windows platform directories.** Windows resolves the roaming and local application
  data directories through the shell API, so `APPDATA` and `LOCALAPPDATA` cannot
  redirect a program that asks the platform.
  The capture store is covered, because the design already gives it
  `UROLLUP_CAPTURE_DIR` ([§2.5](../../urollup-design.md#25-capture-store-and-cache)),
  and the harness sets it.
  The config directory that holds `sources.yaml` and `prices.yaml`
  ([§2.1](../../urollup-design.md#source-manifest)) has no override variable, so on
  Windows a test cannot prove a run ignored a real one.
  A `UROLLUP_CONFIG_DIR` override would close it; that is a maintainer decision, tracked
  as a bead.
- **Harness variable naming.** The runner passes `UROLLUP_BIN`, which sits inside the
  `UROLLUP_*` namespace the design reserves for override variables.
  Nothing collides today, and a rename would touch the gate probes; worth deciding
  before the first override variable ships.

## Practices Adopted From tryscript 0.2.1

The pinned version matters: these are behaviors of 0.2.1, checked in its own reference.

- **Resolution:** `path:` front matter prepends to the inherited `PATH`, so an entry
  that fails to resolve falls through to an installed binary.
  The runner preflights the build and the lint requires `path: [$UROLLUP_BIN]` exactly.
- **Isolation:** `sandbox: true` runs in a fresh temporary directory, and
  `sandbox: <path>` copies that directory there first, which is how a case’s discovery
  root reaches a session without naming an absolute path.
  `[CWD]` then matches the sandbox and treats `/` and `\` as the same separator.
- **Variables:** `env:` and `path:` expand `$VAR` from tryscript’s built-ins and the
  process environment before the command runs, while patterns such as `[CWD]` apply only
  to expected output. This is what lets front matter name paths that commands must not
  spell.
- **Wildcards, in order of preference:** named patterns for typed values, unknown
  wildcards (`[??]`, `???`) only as scaffolding for `--expand`, and generic wildcards
  (`[..]`, `...`) for what genuinely does not matter.
  Patterns over stable fields hide the behavior under test.
- **Rewriting:** `--update` replaces expected output with what it saw, including
  expanded patterns; `--expand`, `--expand-generic` and `--expand-all` fill wildcards by
  category and cannot combine with `--update`.
- **Reporting:** a skipped block counts as passed, an empty selection prints
  `no tests run`, and the summary goes to stdout while per-block results go to stderr.
  Findings 5 and 6 come from these.
- **Project config:** `tryscript.config.ts|js|mjs` is read from the working directory
  where tryscript runs, which is how computed patterns and a harness guard are possible;
  front matter overrides it per file.

## Strategy

Two layers, one fixture corpus, one isolation rule set.
The plan’s
[golden and end-to-end result checks](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#golden-and-end-to-end-result-checks)
records the committed decisions; the reasoning:

- **Transcript goldens answer “what does a user see”.** They hold whole outputs for
  `report`, `daily` and `sessions` in table and JSON form, so a change nobody wrote a
  test for still shows up in a diff.
  Surgical extraction (`grep`, `jq`) is refused by the lint, because it converts a
  session test back into a unit test.
- **Result checks answer “is the number right”.** Each case’s `expected.json` holds
  reconciled truth: unique requests, token categories, ownership counts, copies
  excluded, limit observations, expected diagnostics, and the naive sum for contrast.
  The checker compares only what the case asserts, fails on a result it cannot extract,
  and prints the overcount a naive sum would have produced, which is the reason urollup
  exists ([§1.2](../../urollup-design.md#12-why-urollup-exists)).
- **One corpus, three harnesses.** The same case directories feed the Rust accounting
  tests, these end-to-end checks and the
  [ccusage reconciliation harness](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#ccusage-reconciliation-harness),
  which isolates its runs the same way: temporary copies of the fixture roots, an empty
  `HOME` and XDG directories, native variables naming the copies, and a fixed timezone
  on both sides. Parity compares urollup with another tool; the result checks compare
  urollup with committed truth.
  Neither is an oracle for the other.
- **Ratchets instead of TODOs.** Every state that cannot be checked yet is a pending
  entry naming its bead and its unblock condition, and the run fails as soon as the
  blocker is gone. This is the `general-testing-rules` requirement that an ignored check
  carry an owner and an unblock condition, made executable.

## Real-Log Realism Without Private Data

The maintainer wants fixtures that look like real Claude Code and Codex logs.
Private logs cannot be committed ([SECURITY.md](../../../SECURITY.md), AGENTS.md), so
realism arrives in two steps, neither of which puts a private value in the repository:

- **Structure-only sanitizer.** `scripts/sanitize-claude-fixture.mjs` derives a case
  from a local session: it keeps record types, key names, nesting, ordering, timing
  structure and every usage number, and replaces each string value, identifier and path
  with a synthetic stand-in, mapping consistently so links between records survive.
  The output is reviewed before commit, gets an `expected.json` like any other case, and
  is then an ordinary committed fixture.
  The sanitizer and the cases it derives belong to the fixtures work (bead `uro-obx5`).
- **Local-only aggregate mode.** A consented local corpus is never committed, so it is
  checked in place: `node scripts/check-e2e-results.mjs --fixtures <dir>` already runs
  the same checks over an uncommitted corpus, and the planned local mode adds the case
  with no `expected.json` at all, printing aggregates only, the way `make parity-local`
  does for ccusage: totals per metric, reconciled against naive sums, counts of sessions
  and diagnostics, and nothing else.
  It follows the parity harness’s privacy rules, including a sentinel fixture whose
  paths, project names, IDs and prompt text carry unique markers that must never reach
  output.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
