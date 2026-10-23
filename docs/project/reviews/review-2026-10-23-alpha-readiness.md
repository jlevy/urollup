---
title: Alpha Readiness Review
description: Code review of main at beaede2, real-history runs on the maintainer's machine, and what remains before urollup is usable at alpha quality end to end.
date: 2026-10-23
status: Review complete; findings filed under epic uro-dl9d
---
# Alpha Readiness Review

This review covers `main` at `beaede2` (PR #38, 2026-10-13), the latest commit on
`origin/main` on 2026-10-23. It combines four independent code reviews (Codex adapter,
Claude Code adapter and selection, ledger and query core, sources/admission/CLI), the
full `make check` gate, and consented runs over the maintainer’s real Claude Code and
Codex history. Real-history results are described qualitatively; the private aggregate
values stay local, as `AGENTS.md` requires.

## Verdict

The accounting core is careful, deterministic and well tested, and whole-history runs
work on a real corpus of roughly 19,000 log files.
It is not yet trustworthy end to end, for three reasons that matter more than the open
memory-admission work:

1. **Silent undercounts labeled complete.** Several input shapes produce a smaller total
   with `coverage.complete: true`, no diagnostic and exit 0. One of them, a dangling
   `~/.codex/archived_sessions` symlink, is the maintainer’s configuration today.
2. **Whole-run failures from one bad input.** One unreadable directory, truncated
   sidecar, empty ID or absurd token value makes every command exit 1 for both agents.
   A race with Claude Code creating and deleting files during the walk does the same.
3. **The human output is not yet usable.** Tables are unaligned pipe-separated text, the
   `sessions` table cannot be mapped to a real session, and project grouping splits a
   repository across its worktrees.

None of these needs new architecture.
Most are small, local fixes.
The release plan’s critical path, by contrast, runs through memory-admission slices 5–8,
scale proofs and representative evidence (`uro-6pi8`, `uro-z1h1`, `uro-erqo`), while the
real corpus already completes at about 3% of physical RAM. The recommendation below is
to re-sequence: fix honesty, robustness and presentation first, and keep the existing
row ceiling as the alpha’s safety net.

## What Was Checked

- **Repository state.** `main` fast-forwarded to `origin/main`. Draft PRs #40 and #41
  (admission slices 5 and 6) are green but unchanged since 2026-10-14. The pricing
  engine (money and price table) exists only on `feat/pricing-engine` and the older
  local `codex/alpha-pricing-context`, whose pricing-context half merged as PR #19.
- **Gate.** `make check` passes in full, including all 39 gate probes.
  It needs a workaround when `CARGO_TARGET_DIR` is set: the `parity`, `e2e-local` and
  `parity-local` recipes hard-code `target/debug/urollup` while the golden scripts honor
  the variable, and `gate-proofs` writes about 1.4 GB into the repository’s `target/`
  regardless, which matters on a nearly full disk.
- **Code review.** Four reviewers worked from code and synthetic repros only.
  Every finding marked confirmed below was reproduced with the release binary on
  synthetic input; the most severe were re-run independently.
- **Real history.** `report --current`, whole-history `sessions`, `daily` and
  `report --group-by project,model`, a frozen-input rerun, a one-worker rerun, and the
  pinned ccusage 20.0.20 comparison (`tests/parity/local_diff.py`).

## Real-History Results

- **Every command exits 0.** Whole-history runs took about 40 to 90 seconds each while
  builds and reviews shared the machine, with a peak physical footprint near 1 GB on a
  32 GB machine.
- **Determinism and conservation hold.** On a frozen subset of the corpus (directories
  untouched for six hours), daily sum, report total, sessions sum, per-project sum and
  per-model sum agree exactly for every token class.
  Output is byte-identical across a rerun and across 1 versus 8 workers.
- **Current Claude Code transcript fields decode.** `output_tokens_details`,
  `iterations`, `speed` and one-hour cache writes are read correctly.
- **ccusage agreement is good where both tools see the same files.** The large majority
  of jointly visible sessions match exactly, a few dozen differ by more than 5%, and no
  session is ccusage-only.
  Most of the corpus is visible only to urollup, because ccusage cannot read the
  compressed logs: about two thirds of Claude transcripts and most Codex rollouts are
  `.jsonl.zst`.
- **The archive is silently missing.** `~/.codex/archived_sessions` points at an
  unmounted volume. The run reports no diagnostic and no gap for it; the only trace is
  likely several hundred `thread-orphan` diagnostics for children whose parents were
  archived.
- **Excluded usage is not quantified.** `codex-history-boundary-unverified` fires
  thousands of times and marks the report partial, but no output says how many requests
  or tokens were excluded.
  A user cannot tell a 0.1% gap from a 10% gap.
- **`--current` is slow.** Discovery walks every root before narrowing, taking 2–5
  seconds on this corpus against the plan’s 1-second small-report target.
- **Old Codex formats are harmless but noisy.** Pre-usage rollouts (2025 JSON and early
  JSONL, a handful of 2026 wrapper sessions) contain no token data.
  Default discovery skips them silently, which is fine; naming such a directory with
  `--source` aborts the run with “contains no readable … records”.

## Findings

Locations are on `beaede2`. “Confirmed” means a synthetic repro or test ran; “plausible”
means reasoning only.
Known residuals already listed in the
[release readiness record](../specs/active/plan-2026-09-16-first-release-publishing.md#open-correctness-fixes)
are excluded unless the review found them broader.

### Silent Undercounts Labeled Complete

| # | Finding | Location | Status | Bead |
| --- | --- | --- | --- | --- |
| U1 | A dangling or unmounted `sessions`/`archived_sessions` under a Codex home is dropped before discovery; no missing-root diagnostic or gap | `codex_rollout.rs:1071-1087` (`is_dir`) | Confirmed, and live on the maintainer’s machine | `uro-a1tn` |
| U2 | Symlinks skipped during the walk (broken, outside the root, revisited) are recorded in the manifest, then cleared; design §2.2 says they are reported | `roots.rs:223-253`, `adapters.rs:56` | Confirmed | `uro-0aoi` |
| U3 | Malformed interior Claude lines drop their usage with no diagnostic; Codex emits `malformed-line` but still reports complete | `claude_project.rs:778-824`, `1142-1152` | Confirmed | `uro-yov2` |
| U4 | `daily` and `sessions` documents carry no coverage, unresolved or possible fields; `--session` reports never fill `unresolved`, and `Partial(reasons)` collapses to a bool | `query.rs:333-358`, `aggregate.rs:246-264` | Confirmed | `uro-84j2` |
| U5 | Excluded Codex boundary usage has no magnitude anywhere in the output | `aggregate.rs` coverage summary | Confirmed on real history | `uro-x0rm` |
| U6 | A Claude transcript whose file name is not its `sessionId` (renamed, exported, archived) counts zero, reported as `claude-nested-copy-without-original` | `claude_project.rs:369-377`, `1839-1854` | Confirmed; the real corpus has no such names | `uro-2teg` |
| U7 | A legacy Codex child whose copied prefix never ends (absent parent, or pre-0.100 turns without IDs) has all its own usage excluded as copies | `codex_rollout.rs:1951-1959`, `2201-2202` | Confirmed; broader than `uro-r8si` | `uro-v65i` |
| U8 | Codex format drift degrades to zero: a usage object with no recognized key counts as a request with no tokens, string-typed counts read as absent, unknown record types are skipped, and no `cli_version` gate warns | `codex_rollout.rs:983`, `2791`, `3043-3045` | Confirmed | `uro-4z5o` |

### Double Counts and Merges

| # | Finding | Location | Status | Bead |
| --- | --- | --- | --- | --- |
| D1 | A copied parent `thread_settings_applied` without `thread_id` (Codex 0.144–0.151) hands the rest of the parent’s prefix to the child, counting it twice (synthetic: 6,050 against 3,850, complete) | `codex_rollout.rs:2006`, `2821` | Confirmed; a few real rollouts have both ingredients | `uro-lk0l` |
| D2 | Two Claude responses sharing a `requestId` merge into one request and one is dropped, with only a block-conflict diagnostic | `claude_project.rs:1551-1560`, `1771-1820` | Mechanism confirmed; real frequency unknown | `uro-pw9i` |
| D3 | Codex `LatestRevision` silently picks the last of two differing revisions of one response; the Claude selector reports `revision-disagreement` | `reconcile.rs:182-221` | Confirmed | `uro-ebnx` |
| D4 | Codex twin pairing compares `Option`s strictly while the rest of the adapter reads missing as zero | `codex_rollout.rs:1218`, `1239` | Confirmed; low realism | `uro-d5g9` |

### Whole-Run Failures and Hangs

| # | Finding | Location | Status | Bead |
| --- | --- | --- | --- | --- |
| F1 | Any `NotFound` or `PermissionDenied` during the discovery walk aborts both agents; a create/delete race failed 10 of 30 runs | `roots.rs:135-162`, `claude_project.rs:607-612`, `codex_rollout.rs:719-724` | Confirmed | `uro-57db` |
| F2 | A truncated subagent `.meta.json` sidecar aborts every command | `claude_project.rs:679`, `708-722` | Confirmed | `uro-7zb0` |
| F3 | An empty `requestId`, `message.id` or Codex `response_id` aborts every command | `scope.rs:413` via `observe` | Confirmed | `uro-lx51` |
| F4 | A token value near `u64::MAX` aborts every whole-history command with “token counter overflow” | `aggregate.rs:40`, `239`; `query.rs:175-188` | Confirmed | `uro-3vde` |
| F5 | A `.jsonl` symlink to a FIFO blocks forever in `File::open` | `roots.rs:179-192`, `reader.rs:1062` | Confirmed | `uro-zd3m` |
| F6 | An explicit `--source` holding only usage-free legacy rollouts aborts the run | CLI source validation | Confirmed on real history | `uro-gfl1` |

### Memory

- **M1.** On macOS the line buffer’s shrink-and-regrow pattern grows process memory with
  each line over 8 MiB (`reader.rs:991-997`, plus a fresh `LineBuffer` per scan, peek
  and re-read). Twenty 24 MB lines peaked near 490 MB; `--max-ram 2M` over 24 such lines
  peaked at 822 MB RSS and exited 0. Confirmed; glibc untested (`uro-kpup`).
- **M2.** `--max-ram` is a row ceiling, not a memory cap: the CLI passes
  `MemoryAdmission::unlimited()` (`cli.rs:401`), so slices 1–4 have no user-visible
  effect yet, and the help text overpromises.
  `--max-rows 0` and `--max-ram 8` are accepted and fail at run time instead of as usage
  errors (`uro-r55i`).

### Usability

- Tables are pipe-separated with no alignment or totals row (`render.rs:66`, `89`,
  `112`; `uro-tsxr`).
- The `sessions` table sorts by hashed `thr-` ID and shows neither the native session ID
  nor a date, although the JSON has both (`uro-eqia`). A subagent’s printed selector
  (`<session>/<agent>`) is not accepted by `--session` (`uro-tn3n`).
- Project is the basename of the working directory (`claude_project.rs:946`): every
  worktree becomes its own project (dozens of `agent-*` rows on the real corpus), and
  same-named repositories merge (`uro-f5ss`).
- `report` with no flags is a usage error outside an agent session, and the help does
  not say that `report` defaults to `--current` while `daily` and `sessions` default to
  `--all`. With no logs found, commands print an empty table and exit 0 with no hint
  (`uro-1uye`).
- Errors print I/O kinds in Debug form (`PermissionDenied`, `InvalidInput`), and an
  invalid `TZ` silently becomes `Etc/Unknown` (`uro-1uye`, `uro-o3l4`).
- Missing for parity with what ccusage users expect: cost estimates (`uro-unf7`),
  `--since`/`--until` (`uro-4u5a`), `--agent` (`uro-n4f0`), weekly and monthly views
  (`uro-5qbb`, milestone 0.5).

### Code Health

`observe_parsed_source` in `codex_rollout.rs` is about 830 lines with roughly twenty
interacting mutable flags; D1, U7 and D4 are slips at those interactions.
Splitting ownership placement into its own pass before counter accounting would make
each rule testable on its own.
Elsewhere the code is disciplined: checked arithmetic, no data-reachable panics found,
sorted canonical ordering, and property tests for order independence, idempotence and
worker invariance.

## What Is Sound

- Token arithmetic: disjoint categories, reasoning as a subset, cache writes summed
  once, checked additions everywhere.
- Reconciliation determinism and the conservation of every breakdown.
- Current-format Codex accounting (0.153+ records, root counters, paginated and migrated
  boundaries) and Claude block-revision selection, five-minute/one-hour cache split,
  advisor iterations and `<synthetic>` records.
- Compressed sources: zstd, `--long=27` and gzip read identically to plain files, and
  truncated or corrupt streams degrade to partial coverage with a diagnostic.
- Terminal behavior: progress only on a stderr TTY, `NO_COLOR` respected, JSON never
  colored, a closed stdout pipe treated as success.
- `--current` detection from `CLAUDE_CODE_SESSION_ID` or `CODEX_THREAD_ID`, refusing
  when both are present.

## Path to an Alpha

The [alpha stabilization plan](../specs/active/plan-2026-10-23-alpha-stabilization.md)
maps these findings and every existing bead that gates the frozen alpha scope
(`uro-aakb`) to three phases under epic `uro-dl9d`:

1. **Phase 1 (`uro-q2jo`):** report every loss, degrade per source instead of aborting,
   fix the double counts and silent drops above, and bound the line buffer.
2. **Phase 2 (`uro-eu9x`):** show list-price estimates with pricing coverage, render
   readable tables and a usable sessions view, map worktrees to their project, and add
   the confirmed selection flags.
3. **Phase 3 (`uro-0h0g`):** rerun G1, G2 and G3, the full-history playbook and cost
   parity on the fixed build with the archive mounted, then release.

Supported cost rollups are already in the frozen scope; what was missing was the CLI
work to show them, now `uro-unf7`. Two scope decisions remain open:

- `uro-jzje`: whether the alpha may ship with the row ceiling as its memory guard,
  moving admission slices 7–8, `uro-z1h1` and `uro-erqo` after the first tag.
  Draft PRs #40 and #41 can land whenever they are reviewed; nothing in the real-history
  results suggests memory is the alpha’s main risk.
- `uro-04vk`: whether `--since`/`--until` and `--agent` join the alpha.

Local cleanup, not tracked as beads: the thirteen worktrees under `.claude/worktrees`
and stale local branches (`codex/alpha-pricing-context`, `m01-*`, `worktree-agent-*`).

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
