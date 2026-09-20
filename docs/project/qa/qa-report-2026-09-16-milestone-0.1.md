---
title: Milestone 0.1 QA Report
description: Privacy-safe results from bounded real-log, aggregate, terminal and memory checks for urollup milestone 0.1, including the unaccepted large-default-corpus case.
date: 2026-09-16
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Partial; bounded-corpus and terminal checks passed, while the roughly 15 GB default corpus and local ccusage comparison remain unaccepted
---
# QA Report: Milestone 0.1 on 2026-09-16

## Result

urollup passed the bounded real-log command matrix and terminal behavior checks.
The selected corpus covered both implemented adapters, all measured commands stayed
below an external 1 GiB RSS limit and the emitted machine documents were valid JSON. The
privacy-safe aggregate contained 76 session rows and 6,300 owned requests with no
ambiguous or unknown requests and no diagnostics.

This is not acceptance of the full default corpus.
That corpus is roughly 15 GB, and two unbounded runs over it caused severe memory
pressure, as described under [Crash Investigation](#crash-investigation).
The current compaction work has encouraging bounded and stress-test results.
A 512 MiB decoded-data and one-million-record budget is now enforced while the reader
consumes plain or decompressed input, and a metadata preflight rejects obvious oversized
selections earlier. Against the same default corpus, the guarded release build now exits
with a safety error in under 1.5 seconds at about 11 MiB peak RSS. The full corpus was
intentionally not aggregated without bounded streaming or spill.

## Crash Investigation

Two separate memory incidents occurred during this QA run.
Each was a single top-level `urollup` process built before the ingestion budget, reading
the whole default corpus: about 2.5 GB of Claude Code logs in 1,524 files and 12.4 GB of
Codex logs in 8,756 files.
Neither involved a worker or child-process fan-out.

| Time (PT) | Trigger | Evidence | Effect |
| --- | --- | --- | --- |
| 13:40:55–13:44:48 | The QA command `target/release/urollup daily --all --format json` over default sources | Agent session log; the JSON capture completed and stayed valid at about 1.53 MiB | The maintainer observed roughly 40 GB of memory use, and the Codex desktop app restarted at 13:44:44. macOS wrote no report. |
| 14:11:34 to about 14:14:18 | Accidental: an agent’s `tbd create --description "…"` argument contained a backtick-quoted `urollup daily --all`, so shell command substitution ran the globally installed, pre-budget binary | macOS `JetsamEvent-2026-09-16-142753.ips` and the agent’s command execution records | A 23.2 GB footprint after 79 seconds: 4.2 GB resident and 19.0 GB compressed, with 49.7 CPU-seconds. The compressor held 18.5 GB, 0.38 GB was free and 88 background daemons were jettisoned. The `tbd` command failed after 163 seconds with a stack overflow on the substituted text. |

The second attribution rests on these observations:

- The Jetsam report file is timestamped 14:27:53, but its `timeDelta` of 901 seconds
  places the memory snapshot at 14:12:52; memory pressure delayed the write.
  With that offset, start times derived from each process’s `age` match `ps` start times
  for long-running applications.
- The snapshot shows the `urollup` process as a child of two `bash` processes started
  within 35 ms of it, at 14:11:34.7. The agent recorded its `bash -lc tbd create …`
  command as starting at 14:11:34.9, and no other recorded command started within a
  minute of it.
- The agent’s earlier conclusion that no `urollup` process survived came from a
  sandboxed `ps` that failed with “Operation not permitted,” so it observed nothing.
- The data volume was about 99% full, with roughly 5 GB free, which left little room for
  swap and turned each incident into a machine-wide stall.

A code review also found that both adapters retained more decoded record state than
accounting required, and that default Codex discovery traversed unrelated JSONL files.
Both are fixed in this change.

These safeguards now apply:

- The reader budget and metadata preflight bound ingestion.
  `daily --all`, `sessions --all` and `report --all --group-by project,model` against
  the same default corpus each exit 1 with the safety-limit error in 0.4–1.4 seconds at
  about 11 MiB peak RSS.
- Reinstalling the developer binary from this build bounds any stray `urollup` on
  `PATH`.
- Agents must not place backticks inside double-quoted shell arguments.
  Pass Markdown-formatted text through `--file`, stdin or single quotes.
- The RSS watchdog samples resident memory.
  macOS excludes compressed pages from RSS, so under existing memory pressure the
  observed value can understate a process’s footprint; the 1 GiB limit keeps kills well
  ahead of that regime on an otherwise idle machine.

## Scope and Evidence Boundaries

The bounded corpus contained:

| Adapter | Files | Input Bytes |
| --- | ---: | ---: |
| Claude Code | 44 | 71,916,913 |
| Codex | 32 | 132,265,953 |
| **Total** | **76** | **204,182,866** |

The operator selected the corpus through `$CLAUDE_CONFIG_DIR` and `$CODEX_HOME`. No path
values, session identifiers, prompts, raw records or custom model names are retained in
this report. Each measured process ran under an external 1 GiB RSS watchdog.

The [manual QA playbook](../../../tests/qa/milestone-0.1-local-acceptance.qa.md) defines
the reusable procedure and privacy boundary.

## Test Status

| Area | Result | Evidence |
| --- | --- | --- |
| Bounded command matrix | Passed | Five command forms exited 0; the four measured forms stayed below 116,000 KiB peak RSS |
| Machine output | Passed | Captured documents parsed as JSON; piped output had no ANSI or progress output |
| Interactive terminal behavior | Passed | Automatic color and progress appeared only in the expected terminal cases; disable controls worked |
| Progress cleanup | Passed by automated regression | Unit tests cover cleanup before both successful output and runtime diagnostics |
| Privacy-safe aggregate | Passed with partial coverage | Fixed aggregate fields were produced with no diagnostics; `coverage.complete` was false |
| Typed-compaction stress evidence | Passed for tested cases | Synthetic skipped Codex, largest selected Claude project and a 204 MiB Codex day stayed within measured bounds |
| Default-corpus safety guard | Passed | Three `--all` forms over the roughly 15 GB default corpus exited 1 with the safety-limit error at about 11 MiB peak RSS |
| Full roughly 15 GB default corpus | Not Run | The ingestion budgets prevent another unbounded run; bounded `--all` streaming or spill is not implemented |
| `make parity-local` on default logs | Not Run | No completed, privacy-safe local comparison record was available for this report |

## Bounded Command Results

Every command used explicit bounded roots, disabled default-source discovery, requested
JSON and ran beneath the 1 GiB watchdog.
Selectors are intentionally omitted.

| Command Form | Exit | Peak RSS (KiB) | Elapsed Seconds | Result |
| --- | ---: | ---: | ---: | --- |
| `sessions --all --format json` | 0 | 109,776 | 2 | Valid JSON |
| `daily --all --format json` | 0 | 108,736 | 1 | Valid JSON |
| `report --all --group-by model,project --format json` | 0 | 112,400 | 2 | Valid JSON |
| Exact session with descendant scope and JSON | 0 | 115,632 | 2 | Valid JSON |
| `report --current --format json` | 0 | Not recorded | Not recorded | Valid JSON |
| `report --current --format json`, default sources | 0 | 10,928 | 0.4 | Valid JSON; 42 owned requests, no diagnostics |

The guard checks below used the default sources instead of the bounded roots.
They record the intended failure, not an accepted aggregate:

| Command Form, Default Sources | Exit | Peak RSS (KiB) | Elapsed Seconds | Result |
| --- | ---: | ---: | ---: | --- |
| `daily --all --format json` | 1 | 10,976 | 1.4 | Safety-limit error before parsing |
| `sessions --all --format json` | 1 | 10,688 | 0.4 | Safety-limit error before parsing |
| `report --all --group-by project,model --format json` | 1 | 10,672 | 0.4 | Safety-limit error before parsing |

## Privacy-Safe Aggregate

The aggregate is intentionally limited to fixed counters and contains no raw event or
identity fields.

| Measure | Value |
| --- | ---: |
| Session rows | 76 |
| Daily rows | 4 |
| Owned requests | 6,300 |
| Ambiguous requests | 0 |
| Unknown requests | 0 |
| Diagnostics | 0 |

### Token totals

| Token Class | Tokens |
| --- | ---: |
| Uncached input | 9,655,597 |
| Cache read | 1,359,173,325 |
| Cache write, five-minute tier | 19,859,886 |
| Cache write, one-hour tier | 7,463,975 |
| Cache write, unspecified tier | 0 |
| **Cache write, total** | **27,323,861** |
| Output | 5,183,690 |
| Reasoning, a subset of output | 2,218,821 |
| **Total** | **1,401,336,473** |

### Coverage and request sizes

Coverage was partial, not complete.
The aggregate recorded 25 excluded copies, 214 limit observations and 2 requests without
usage; it recorded no ambiguous or unknown ownership.

| Request-Size Measure | Inclusive Input Tokens |
| --- | ---: |
| Count | 6,300 |
| p50 | 189,023 |
| p90 | 401,344 |
| p99 | 844,315 |
| Maximum | 966,277 |

## Terminal Behavior

The pseudo-terminal checks observed these behaviors:

- Automatic help output exited 0 and emitted 45 SGR sequences.
- Automatic `sessions` output exited 0, emitted 3 SGR sequences and showed progress on
  stderr.
- `--color never --no-progress` emitted neither color nor progress.
- Piped output emitted neither ANSI sequences nor progress.
- `NO_COLOR=1` suppressed SGR sequences while preserving interactive progress.
- A nonexistent selector exited 1.

Automated CLI regression tests separately prove that progress is written only to stderr
and is cleared before both successful output and runtime diagnostics.
Machine-format tests prove that JSON stays plain and suppresses progress even when color
is explicitly requested.

## Large-Corpus Safety Limitation

The roughly 15 GB default corpus is not accepted.
An earlier unbounded release-binary run caused severe memory pressure and restarted the
task, so repeating it without a hard safety boundary would not be responsible.

The current typed-compaction changes remove raw-payload retention.
The shared reader budget stops after 512 MiB of decoded input or one million records,
including input that expands beyond the limit after zstd decompression.
A metadata preflight rejects obvious oversized selections before parsing.
Three smaller stress checks also passed:

| Case | Input | Result |
| --- | ---: | --- |
| Synthetic skipped Codex input | 134,217,728 bytes | 0.04 seconds; 3,801,088 bytes peak RSS |
| Largest selected Claude project | 472,936,448 bytes; 136,262 records | Approximately 443,904 KiB peak RSS |
| Selected Codex day | Approximately 204 MiB | Approximately 97,760 KiB peak RSS |

These checks show material improvement but do not establish a bounded implementation for
the full default corpus.
Milestone 0.1 still needs streaming or spill within its memory target, followed by a
guarded full-corpus rerun.
Until then, full-default-corpus acceptance and local parity over that corpus remain
**Not Run**, not passed.

## Disposition

The bounded command, JSON, terminal and privacy-safe aggregate checks pass.
Milestone 0.1 is not fully accepted because the large-default-corpus safety case and the
consented local ccusage comparison remain open.

Before final acceptance:

1. Land and validate the typed-compaction changes through `make check` and independent
   implementation review.
2. Keep selection-aware narrowing and the temporary safety barrier, then implement
   bounded streaming or spill for the full default corpus.
3. Rerun the full corpus only behind that guard and record privacy-safe peak memory,
   elapsed time and aggregate results.
4. Complete `make parity-local CONSENT_LOCAL_LOGS=1`, then explain every threshold
   exceedance or unexplained residual without copying private values.

## Related Documents

- [Milestone 0.1 Local Acceptance QA](../../../tests/qa/milestone-0.1-local-acceptance.qa.md)
- [urollup Design](../../urollup-design.md)
- [Milestone 0.1 Implementation Plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md)
- [Golden Testing Audit](../research/research-2026-09-15-golden-testing-audit.md)
- [Parity Harness](../../../tests/parity/README.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
