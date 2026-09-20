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
- Never generate more than 512 MiB in one corpus without checking free disk space first.
  The engine no longer limits input size, but disk space on a typical development
  machine is limited, and the default sweep (32/64/128/256 MiB) already shows the
  scaling slope.

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

### Usage-record density

The generator writes usage records far more densely than real logs do.
With default (unpadded) content it produces about 660 usage records per MiB of generated
content.
Real Claude Code logs carry about 130 usage records per MiB, and real Codex logs
about 40 per MiB — both far sparser, since a real transcript spends most of its bytes on
prompts, replies and tool output that accounting never reads.

This matters because the engine’s
[Memory Model](../specs/active/plan-2026-09-16-scalable-ingestion.md#memory-model)
tracks peak footprint against the number of usage-bearing records, not raw bytes.
A synthetic corpus sized by `--max-bytes` alone is therefore not a stand-in for “this
many MiB of real logs” — it is denser, so it carries more usage records per MiB than the
real thing. Whenever record count (not corpus size) is what matters, read it from the
generator’s own JSON summary (`usage_records`) rather than assuming a fixed
records-per-MiB ratio from `--max-bytes`.

`--content-padding-bytes` is the generator’s density control: it appends bytes to fields
the engine never reads for accounting, which lowers records-per-MiB without changing the
usage-record count at all.
Passing a larger value approximates real logs’ lower density (for example,
`--content-padding-bytes 5000` with `--max-bytes 0` and small day/session counts
produces a corpus with far fewer usage records per MiB of content than the default).
This is also what makes the raw-bytes independence check possible: two corpora at very
different densities but identical usage-record counts.
`scripts/check-scale.py` (below) relies on both properties: it fits its extrapolation
bound against `usage_records`, and its independence check holds `usage_records` fixed
while varying padding.

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
  A corpus above the engine’s compact-row ceiling (default 25% of RAM, or 2 GiB when RAM
  cannot be read) is reported as a clean refusal (the row’s status column reads
  `refused`), not treated as a crash.
  This is an observation-row admission limit, not a process-memory cap; corpus bytes
  alone do not establish headroom.
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
status of `ok`, `refused` (hit the engine’s compact-row capacity ceiling) or `killed`
(hit the watchdog’s kill-switch limit — treat this as a real regression, not noise).
The fitted slope is peak memory bytes per input MiB; the intercept is a fixed baseline
overhead. Compare slopes across urollup builds (`--binary`) to see whether a change to
the data model actually reduced the bytes-per-input-MiB ratio the
[Goals](../specs/active/plan-2026-09-16-scalable-ingestion.md#goals) call for.

## CI scale gate

`scripts/check-scale.py` turns three of the plan’s
[Phase 2](../specs/active/plan-2026-09-16-scalable-ingestion.md#phase-2-fast-decode-and-scale-gates)
requirements into pass/fail assertions, run by `make scale-gate` (part of `make test`).
Dedicated Ubuntu/macOS CI jobs execute the release workload after the supply-chain gate
and archive `scale-gate.log`, retaining failure status through the logging pipeline.
These are small synthetic regression checks, not whole-history acceptance:

1. **Raw-bytes independence.** Two corpora with identical usage records but very
   different content padding must have peak-memory footprints within
   `--independence-ceiling-mib` of each other (default 64 MiB — the plan’s own bound).
   A larger difference means memory is scaling with raw log bytes, not usage records.
2. **Footprint extrapolation bound.** Peak memory at a few small corpus sizes (default
   4, 8, 16 MiB; see [Usage-record density](#usage-record-density) for why sizes are
   expressed in MiB but the fit is not) is fit to
   `peak_bytes ~= intercept + slope * usage_records`, and both the fitted slope
   (`--extrapolation-slope-ceiling-bytes-per-record`) and intercept
   (`--extrapolation-intercept-ceiling-mib`) must stay under calibrated ceilings.
3. **`daily --all` under the RSS watchdog at 512 MiB.** A smoke test — a generated
   corpus (`--daily-gate-mib`, default 32 MiB) run through `daily --all` under
   `scripts/run-rss-watchdog.py` with a 512 MiB kill switch, matching the plan’s
   whole-history footprint goal.

Run it directly, or through the Make target that builds the release binary it needs:

```bash
make scale-gate
# or, against an already-built binary:
uv --config-file uv.toml run --frozen python scripts/check-scale.py \
  --binary target/release/urollup
```

Like the other scripts here, it never reads real logs (`--no-default-sources` always),
generates at most 256 MiB in one corpus, and deletes every corpus as soon as it has been
measured. The whole gate — all three checks — takes under a minute on an unloaded
machine.

If the engine refuses a corpus at its compact-row capacity ceiling, the refusal is
reported as its own clearly labeled failure, not folded into a generic non-zero-exit
message. When a run refuses a corpus, inspect the diagnostic’s actual budget and
environment before attributing it to a regression.
Explicit overrides can intentionally make even a small corpus exceed the row ceiling.

### Calibrated thresholds

The default ceilings were calibrated on 2026-09-17 against the Phase 1 engine at commit
`4c0daae` (with its since-removed 512 MiB input guard raised; the guard did not affect
these small default sizes), on an Apple M1 Pro under ordinary developer load:

| Check | Measured | Default ceiling | Margin |
| --- | ---: | ---: | ---: |
| Raw-bytes independence (peak difference, identical usage records) | ~1.0–1.5 MiB | 64 MiB | ~45x (fixed by the plan, not tuned) |
| Extrapolation slope (bytes per usage record) | ~3,000–4,100 B/record | 8,192 B/record | ~2–2.7x |
| Extrapolation intercept (fixed baseline) | ~3.5–10.5 MiB | 48 MiB | ~5–14x |
| `daily --all` peak at 32 MiB corpus (~21,000 usage records) | ~85–95 MiB | 512 MiB (watchdog) | ~5.5x |

The extrapolation slope was the most stable of these across repeated runs (it varied by
only a few percent); the intercept was noisier, since it is dominated by fixed process
and allocator overhead rather than by anything the corpus controls.
The margins above are deliberately generous for a machine shared with other work, per
the safety rules above — they are meant to catch a real regression (an accidental return
to per-record-bytes-proportional retention, or a superlinear growth in record count),
not to pin the current engine’s numbers exactly.

**These numbers are from macOS (`/usr/bin/time -l`, “peak memory footprint”).** Linux’s
`/usr/bin/time -v` reports “Maximum resident set size” instead, a different (typically
larger) accounting that does not exclude the same categories of pages.
CI now runs both platforms.
Retain each job’s measurements and validate its thresholds against that platform’s
accounting; the historical macOS calibration is not Linux calibration.
The watchdog separately samples aggregate process-group RSS, so its peak can miss short
spikes and must not be equated with either OS peak metric.

### Recalibrating

1. Build (or obtain) the release binary you want to calibrate against.

2. Run the gate once with generous ceilings and read the printed measurements — for
   example:

   ```bash
   uv --config-file uv.toml run --frozen python scripts/check-scale.py \
     --binary target/release/urollup \
     --extrapolation-slope-ceiling-bytes-per-record 1000000 \
     --extrapolation-intercept-ceiling-mib 1000 \
     --independence-ceiling-mib 1000
   ```

   The `[PASS]`/`[FAIL]` summary lines print the fitted `peak MiB ~= intercept + slope *
   records` line and the independence peak difference directly.

3. Repeat a few times (the machine’s load affects the intercept more than the slope; see
   above) and pick new ceilings with a margin over the worst observed run, following the
   same reasoning as the table above.

4. Update the relevant `--extrapolation-*`/`--independence-ceiling-mib` defaults in
   `scripts/check-scale.py`’s `parse_args`, and update the table above with the new
   measurements and the date, binary and machine they came from.

### Demonstrating a failure

Passing an unreasonably strict ceiling proves the gate actually checks something, rather
than always passing:

```bash
uv --config-file uv.toml run --frozen python scripts/check-scale.py \
  --binary target/release/urollup \
  --extrapolation-slope-ceiling-bytes-per-record 10 \
  --independence-ceiling-mib 0.01
```

This exits 1 and prints `[FAIL]` lines naming the violated bound and the measured value,
for example `slope 4044 B/record exceeds the 10 B/record ceiling`.

## Tests

`tests/qa/test_synthetic_corpus.py` (discovered by `make qa-tool-tests`) checks the
generator’s determinism (the same seed produces byte-identical output trees) and that it
never exceeds `--max-bytes`, at small sizes so the suite stays fast.
It does not run `measure-scale.py` itself, which needs a release build and takes real
wall time; run it manually as shown above.

`tests/qa/test_check_scale.py` checks `check-scale.py`’s pure decision logic —
corpus-size parsing, the peak-memory-versus-usage-records line fit, and the pass/fail
judgment for each of the three checks (including that an engine refusal or a watchdog
kill is reported as its own clearly labeled failure) — without running a binary or
generating a corpus, so it stays fast.
It does not run `check-scale.py` itself end-to-end; that needs a release build, and is
instead exercised by `make scale-gate`.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
