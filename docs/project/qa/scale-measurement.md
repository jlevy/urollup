---
title: Scale Measurement
description: How to generate a synthetic Claude Code and Codex corpus and measure how urollup's wall time and peak memory scale with input size, without reading anyone's real logs.
date: 2026-09-16
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Active
---
# Scale Measurement

This explains how to reproducibly measure how urollup’s memory and wall time scale with
input size, using a synthetic corpus rather than anyone’s real Claude Code or Codex
logs. It supports the
[scalable ingestion plan](../specs/active/plan-2026-09-16-scalable-ingestion.md), whose
Memory Model and Phase 2 testing strategy this tooling exists to validate.

Two scripts do the work:

- `scripts/generate-synthetic-corpus.py` streams a realistic-looking Claude Code
  `projects/` tree and Codex `sessions/` tree into an output directory, under a byte
  cap, deterministic from a seed.
- `scripts/measure-scale.py` runs the generator at several sizes, measures `sessions`,
  `daily` and `report` under `/usr/bin/time` and a kill-switch watchdog, and prints a
  table with a fitted memory-per-input-byte slope.

Both are development-only Python, standard library only, run through
`uv --config-file uv.toml run --frozen python …` per [AGENTS.md](../../../AGENTS.md).

## Safety rules

These are not suggestions; the milestone 0.1 QA report documents two incidents where an
unbounded run over the real default corpus reached 23–40 GB and stalled a machine.

- **Never** point either script, or a manually run `urollup`, at `~/.claude` or
  `~/.codex`, and never run `urollup` without `--no-default-sources` while testing.
  Both scripts already pass `--source`/`--no-default-sources`; do not remove that.
- Keep generated corpora in a temporary directory outside the repository (both scripts
  do this by default) and delete them after use.
  `measure-scale.py` deletes each corpus itself as soon as it finishes measuring that
  size.
- Never generate more than 512 MiB in one corpus.
  The current engine refuses any discovered input it estimates at more than 512 MiB
  (`crates/urollup/src/cli.rs`), so there is nothing to measure above that, and disk
  space on a typical development machine is limited.
  The default sweep (32/64/128/256 MiB) stays comfortably under the limit.

## Generating a corpus

```bash
uv --config-file uv.toml run --frozen python scripts/generate-synthetic-corpus.py \
  --out /tmp/urollup-synthetic-corpus \
  --seed 1 \
  --max-bytes $((256 * 1024 * 1024))
```

This writes `<out>/claude/projects/…` and `<out>/codex/sessions/…` and prints a JSON
summary of files, bytes and usage records written.
Point urollup at it directly:

```bash
cargo build --release --locked --workspace
target/release/urollup report --all \
  --source /tmp/urollup-synthetic-corpus/claude/projects \
  --source /tmp/urollup-synthetic-corpus/codex \
  --no-default-sources --format json
```

(`--source` classifies a directory containing `projects/` as a Claude root and one
containing `sessions/` or `archived_sessions/` as a Codex home; see `urollup --help`.)

### What it generates

Records are modeled on the shapes in
`crates/urollup-core/tests/fixtures/{claude-project,codex-rollout}/*/README.md`:

- **Claude:** sessions with subagents (including nested workflow subagents) and
  `.meta.json` sidecars; assistant records with usage, including repeated block records
  sharing one `message.id`; resumed sessions that replay another session’s `uuid` and
  `sessionId`; `progress` records nesting a real subagent message; and `quotaLimits`.
- **Codex:** rollouts named `rollout-<timestamp>-<uuid>.jsonl` under
  `sessions/YYYY/MM/DD`; `session_meta`, `turn_context` and cumulative `event_msg`
  `token_count` records with `rate_limits`; forked rollouts with copied history; and
  both `token_usage_record` and legacy-counter-only rollouts.

### Key options

| Option | Purpose |
| --- | --- |
| `--seed` | Determinism: the same seed and options always produce byte-identical output. |
| `--max-bytes` | Streaming byte cap across both agents (default 256 MiB). `0` means unlimited, bounded only by `--days` and the per-day counts — used for corpora that must have a fixed, seed-reproducible record count regardless of content size. |
| `--claude-fraction` | Share of `--max-bytes` given to Claude content; the rest goes to Codex (default 0.2, roughly the maintainer corpus’s real split). |
| `--content-padding-bytes` | Extra bytes appended to content the engine never reads for accounting (prompts, replies, tool descriptions). Raises raw corpus bytes without changing usage numbers — see [Raw-bytes independence](#raw-bytes-independence) below. |
| `--zstd-fraction` | Fraction of Codex rollouts written as `.jsonl.zst` instead of `.jsonl`. Requires a `zstd` binary on `PATH`; the script exits with a clear error if one is requested but not found. |
| `--resume-fraction`, `--subagent-fraction`, `--workflow-fraction`, `--quota-fraction`, `--fork-fraction`, `--token-usage-record-fraction` | Shape controls for how much of the corpus exercises each of the features above. |
| `--days`, `--claude-sessions-per-day`, `--codex-rollouts-per-day` | Controls for the number of sessions and days spanned. Left unset, they are derived from `--max-bytes` so the byte cap — not a session-count ceiling — is normally what stops generation. |

Run `--help` for the full list, including `--claude-projects` (how many distinct project
directories to rotate through).

### A note on the 64x zstd multiplier

The 512 MiB engine safety check estimates a `.zst` source’s decoded size as 64 times its
compressed size on disk (a deliberately conservative bound).
A corpus that is mostly zstd-compressed can therefore trip the safety refusal well
before its logical `--max-bytes` content would, even though very little real disk space
was used. This is occasionally useful for exercising the refusal path cheaply (see
`tests/qa/test_synthetic_corpus.py` and the harness’s own handling of a refusal), but it
means `--zstd-fraction` should stay modest for a corpus meant to actually be ingested.

## Measuring scale

```bash
uv --config-file uv.toml run --frozen python scripts/measure-scale.py
```

With no arguments this generates 32, 64, 128 and 256 MiB corpora (one at a time,
deleting each before moving to the next), runs `sessions --all`, `daily --all` and
`report --all` on each under `scripts/run-rss-watchdog.py` (a 2048 MiB kill switch) and
`/usr/bin/time -l` (macOS) or `/usr/bin/time -v` (Linux), and prints a table plus a
fitted `peak_bytes ~= slope * input_MiB + intercept` line per command.
It finishes with a raw-bytes independence check (see below).

Useful flags:

- `--binary PATH` — measure a different urollup build (debug, an alternate branch’s
  release binary, and so on).
  Defaults to `target/release/urollup`.
- `--sizes-mib 32,64,128` — a custom size list.
  Sizes above 512 MiB are reported as a clean refusal (the row’s status column reads
  `refused`), not treated as a crash.
- `--watchdog-limit-mib` — the kill-switch limit (default 2048 MiB); lower it if you
  want to catch a regression sooner rather than let it run to the OS limit.
- `--skip-independence` — skip the raw-bytes independence check, e.g. for a quick
  scale-only run.

### Raw-bytes independence

The [Memory Model](../specs/active/plan-2026-09-16-scalable-ingestion.md#memory-model)
requires that peak footprint depend on the number of usage-bearing records, not on raw
log bytes. `measure-scale.py` checks this directly: it generates two small corpora with
the same seed and the same fixed session/rollout counts (so they have identical usage
records), differing only in `--content-padding-bytes` — one at a baseline padding and
one at 10x that padding — and reports the difference in peak memory between them.
A large difference despite identical usage records would indicate memory is scaling with
raw bytes rather than with decoded records.

### Reading the output

Each row reports the corpus’s usage-record count (not just its nominal size, since
record density varies with the shape controls) alongside wall time, peak memory and a
status of `ok`, `refused` (hit the 512 MiB engine safety limit) or `killed` (hit the
watchdog’s kill-switch limit — treat this as a real regression, not noise).
The fitted slope is peak memory bytes per input MiB; the intercept is a fixed baseline
overhead. Compare slopes across urollup builds (`--binary`) to see whether a change to
the data model actually reduced the bytes-per-input-MiB ratio the
[Goals](../specs/active/plan-2026-09-16-scalable-ingestion.md#goals) call for.

## Tests

`tests/qa/test_synthetic_corpus.py` (discovered by `make qa-tool-tests`) checks the
generator’s determinism (the same seed produces byte-identical output trees) and that it
never exceeds `--max-bytes`, at small sizes so the suite stays fast.
It does not run `measure-scale.py` itself, which needs a release build and takes real
wall time; run it manually as shown above.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
