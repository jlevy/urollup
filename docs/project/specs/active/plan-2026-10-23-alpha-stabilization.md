---
title: Alpha Feature-Complete Stabilization
description: Every remaining bead between main and a trustworthy, feature-complete 0.1.0 alpha, grouped into stabilization, completion and acceptance phases.
author: Joshua Levy with LLM assistance
date: 2026-10-23
status: Active; beads filed, two scope decisions open; in-flight work rechecked 2026-11-14
---
# Feature: Alpha Feature-Complete Stabilization

## Overview

This plan maps all remaining work for the first public alpha, urollup 0.1.0, to beads
and phases. The scope is the maintainer’s frozen alpha slice (`uro-aakb`): reliable
per-session and whole-machine Claude Code and Codex usage, with supported list-price
cost rollups and explicit incomplete and unpriced coverage.
The work comes from the
[2026-10-23 alpha readiness review](../../reviews/review-2026-10-23-alpha-readiness.md)
together with every open bead that already gates that slice.

The epic is `uro-dl9d`. Phase 1 (`uro-q2jo`) makes every loss visible, keeps one bad
input from failing a run, and fixes the known double counts.
Phase 2 (`uro-eu9x`) adds cost estimates and output a person can read.
Phase 3 (`uro-0h0g`) reruns acceptance and releases.
`uro-aakb` depends on the epic, and milestone 0.1 (`uro-n8h5`) depends on Phases 1 and
2\.

This plan sequences work and owns no design.
Existing owners keep their designs:

- the [design specification](../../../urollup-design.md) for behavior;
- the [scalable-ingestion plan](plan-2026-09-16-scalable-ingestion.md) for memory
  admission;
- the [usage-analysis plan](plan-2026-09-20-usage-analysis-workflow.md) and design §4.5
  for pricing;
- the [publishing plan](plan-2026-09-16-first-release-publishing.md) for release
  machinery and the readiness record.

## Goals

- **Every loss is visible.** No supported input shape produces a smaller total with
  `coverage.complete: true` and no diagnostic.
  Every report, daily and sessions document says when it is partial, why, and how much
  usage the gaps exclude.
- **One input never fails a run.** A missing, unreadable, malformed, racing or absurd
  source never aborts or hangs a run for everyone.
  It becomes that source’s coverage gap.
- **Fixed double counts stay fixed.** The double counts and silent drops found on
  `beaede2` are fixed, each with a synthetic regression.
  Known residual shapes are either fixed or documented as limitations in the release
  notes.
- **Memory stays manageable.** A run on a real history stays within the accepted memory
  policy. The line buffer no longer grows process memory on macOS.
- **Costs are supported.** `report`, `daily` and `sessions` show a list-price estimate
  with priced, default-assumed and unpriced coverage.
- **The everyday CLI is usable.** Tables are aligned, with totals.
  The sessions view identifies real sessions.
  Projects survive worktrees.
  Help states each command’s default selection.
- **Acceptance passes on the fixed build.** G1, G2 and G3, the full-history QA playbook
  and the token and cost parity harness all pass, followed by the 0.1.0 release
  checklist.

## Non-Goals

These keep their existing owners and do not gate the alpha:

| Scope | Owners |
| --- | --- |
| Weekly, monthly, requests, tools, tree and sources commands; JSONL, CSV and Markdown output; query files and `--strict` | `uro-5qbb`, `uro-81ku`, `uro-9cvc` |
| Selection flags other than `--agent`: `--hook-input`, `--sessions-from`, `--latest`, `--project`, `--cwd`, `--whole-sessions` | `uro-feyo`, `uro-le2e` |
| Report presentation candidates beyond aligned tables: `--compact`, `--no-cost`, `--last`, pooled cache share | `uro-00fc` |
| User price overrides and `prices.yaml` | `uro-640p`, unless the pricing question below adds it |
| Summaries, bundles, export and merge | `uro-v858` and its children |
| Capture store | `uro-xr5l` and its children |
| Joint grouping, reusable snapshots, tools and time, G5 | `uro-6kwn`, `uro-qvp1`, `uro-x8r8`, `uro-f2lv`, `uro-i6xb` |
| Analytical tables and HTML pivots | `uro-ah7a` |
| Web UI, serve, G4, Pi and Gemini CLI adapters | `uro-xbgf` and its children |
| Cursor | `uro-hdoq` and its children |
| Status line | `uro-vccm`, `uro-iui3` |
| Hybrid spill and decode throughput | `uro-924y`, `uro-lsaz`, `uro-6gwt`, `uro-nzo1` |
| Benchmarks and the ccusage/agentfdr feature matrix | `uro-2kd6`, `uro-sbnk`, `uro-hu9s`, `uro-o65n`, `uro-ey63` |

## Background

On `beaede2` the accounting core is careful and deterministic.
Whole-history runs on the maintainer’s corpus finish in about a minute, at roughly 1 GB
peak footprint. Every report agrees with every other on frozen input, and output is
byte-identical across worker counts.
Where both tools can read the logs, urollup agrees with pinned ccusage for the large
majority of sessions.
The review found three kinds of problems instead:

- **Silent undercounts.** Some inputs give a smaller total with `coverage.complete:
  true`. One of them is live on the maintainer’s machine: an unmounted archive volume.
- **Whole-run failures.** One bad file or value makes every command exit 1, and a FIFO
  link hangs a run.
- **Output a person cannot use.** Tables are unaligned, sessions are unidentifiable, and
  each worktree is its own project.

The frozen scope already requires cost rollups, memory safety and acceptance.
It does not yet include the review’s findings or the cost display in the CLI. This plan
adds both and maps them, with every existing gating bead, to one sequence.

### In-Flight Work

A 2026-11-14 check found `main` still at `beaede2` and nothing pushed since 2026-10-14.
No branch, worktree or closed bead fixes any finding in this plan.
Four pieces of in-flight work cover parts of it and should be reused, not redone:

- **Admission slices 5 and 6:** draft PRs #40 and #41 are green on top of `beaede2`
  (`uro-6pi8`).
- **Admission slice 7:** uncommitted work in the local `admission-slice7` worktree
  (`feat/admission-cli`) charges rendered output to admission and adds footprint stats.
  It still passes the unlimited ledger with a row ceiling, so `uro-r55i` lands with or
  after it.
- **Pricing foundation:** `feat/pricing-engine` commit `7d9a242` holds the money and
  price-table foundation for `uro-wuby` and `uro-unf7`. The branch’s other commits are
  earlier versions of work merged in PR #19.
- **`--agent`:** the draft Cursor branch `cursor-state-adapter` implements the flag
  inside its Cursor commit `e801c7c`. `uro-n4f0` can lift the CLI part without the
  Cursor reader. Both branches are 69 commits behind `main`.

## Design

### Approach

1. **One coverage block.** `report`, `daily`, `sessions` and selected reports share one
   coverage structure. It holds completeness, partial reasons, gap counts by kind, the
   excluded request and token magnitudes, unresolved and possible side totals, and
   requests without usage.
   Sessions rows carry a partial flag.
2. **Degrade per source.** Usage errors, explicitly named missing roots and internal
   invariant failures exit non-zero.
   Anything else wrong with one input becomes that input’s coverage gap and a
   diagnostic: unreadable paths, races, malformed lines or sidecars, empty IDs,
   implausible counts, skipped links and dangling default roots.
3. **Correctness fixes carry evidence.** Each accounting fix lands with a synthetic
   regression, an end-to-end fixture golden where the shape is realistic, and a
   format-contract update in `docs/formats/`.
4. **Visible drift.** Each adapter records its newest tested agent version and emits one
   diagnostic for newer sources and for unknown usage-bearing records.
5. **Output a person can read.** Use aligned, width-aware tables with totals.
   The sessions view shows native ID, dates and cost.
   Projects follow the design §2.1 recorded-field rules, so a worktree maps to its
   repository.

### Components

- **Coverage and query:** `urollup-core/src/query.rs` and `query/aggregate.rs`, plus
  `accounting/totals.rs`.
- **Discovery and reading:** `sources/roots.rs` and `sources/reader.rs`, plus each
  adapter’s discovery entry point.
- **Accounting fixes:** `adapters/codex_rollout.rs`, `adapters/claude_project.rs` and
  `ledger/reconcile.rs`.
- **Pricing:** `accounting/` money and price-table modules, starting from
  `feat/pricing-engine`.
- **CLI:** `urollup/src/cli.rs` and `render.rs`.

### API Changes

- **JSON (bump `schema_version`):**
  - `daily` and `sessions` gain the coverage block.
  - Every coverage block gains partial reasons, gap kinds and excluded magnitudes.
  - Report totals split self and descendant usage.
  - Cost fields carry the pricing basis and coverage.
  - `uro-2025` records the contract version in `CHANGELOG.md`.
- **CLI flags:** `--since` and `--until`, and `--agent`, subject to `uro-04vk`.
- **Validation:** `--max-ram` and `--max-rows` are checked as usage errors, and their
  help says what they bound.
- **Exit codes:** unchanged.
  Partial results still exit 0. `--strict` and `--require-priced` keep their owners;
  `uro-7nhm` adds `--require-priced`.

## Implementation Plan

Classes: **Gate** must be closed before the Phase 3 acceptance reruns.
**Fix or document** ships fixed or as a known limitation in the release notes
(`uro-2025`). **Decide** must be recorded, either resolved or explicitly deferred.
Phases 1 and 2 touch different code and can proceed in parallel.

### Phase 1: Honest Coverage, Robust Ingestion and Correct Accounting

Epic `uro-q2jo`. New beads are its children.
Existing beads in scope block it.

**Coverage honesty**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-84j2` | Coverage, unresolved usage and partial reasons in daily, sessions and selected reports | Gate |
| `uro-x0rm` | Quantify usage excluded as coverage gaps (after `uro-84j2`) | Gate |
| `uro-1vg5` | Session selection drops request-scoped diagnostics | Gate |
| `uro-akgu` | Codex boundary diagnostic names no session | Gate |
| `uro-a1tn` | Report dangling or unmounted Codex session roots | Gate |
| `uro-0aoi` | Report symlinks skipped during discovery | Gate |
| `uro-yov2` | Claude malformed lines skipped without a diagnostic; Codex malformed lines leave coverage complete | Gate |
| `uro-4z5o` | Warn when log formats drift past the tested versions | Gate |
| `uro-pny9` | Attach the owning thread to unreadable-source gaps | Fix or document |
| `uro-h6m9` | Diagnostic rows keep one adapter’s detail | Fix or document |

**Robustness**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-57db` | Discovery-walk errors and races become gaps | Gate |
| `uro-7zb0` | An unreadable subagent sidecar loses only linkage | Gate |
| `uro-lx51` | Empty request and response IDs read as absent | Gate |
| `uro-3vde` | Implausible token counts are excluded, not fatal | Gate |
| `uro-zd3m` | Vet non-regular files at sidecar opens and link targets (FIFO hang) | Gate |
| `uro-o3l4` | Named time zones on Windows; invalid or unknown default zone | Gate |
| `uro-gfl1` | Explicit `--source` holding only usage-free legacy logs | Fix or document |
| `uro-hcgb` | Single subagent file named by `--source` keeps its identity | Fix or document |
| `uro-wwrr` | Plain log versus a more complete compressed twin | Fix or document |
| `uro-tn3n` | `--session` accepts the printed subagent selector | Fix or document |
| `uro-4v0g` | Key Claude selection families by session ID | Fix or document |

**Accounting correctness**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-lk0l` | Unnamed copied settings event hands a Codex parent’s prefix to the child | Gate |
| `uro-v65i` | Unended legacy Codex prefix drops the child’s own usage | Gate |
| `uro-ebnx` | Report revision disagreement under Codex `LatestRevision` | Gate |
| `uro-pw9i` | Claude responses sharing a `requestId` merge | Gate if real records share IDs, else fix or document |
| `uro-2teg` | Claude transcript whose name is not its `sessionId` counts zero | Fix or document |
| `uro-r8si` | Codex forks and subagents drop counter-only usage; nested legacy chains recount | Fix or document |
| `uro-eh0d` | Codex child recounts a rolled-back parent turn | Fix or document |
| `uro-p12c` | Native counter-only Codex forks recount the seeded total | Fix or document |
| `uro-uwv0` | A fork’s own step repeating a parent total is dropped | Fix or document |
| `uro-zq65` | Seeded bounded-migrated child reports its seed as a gap | Fix or document |
| `uro-3b12` | Nested `thread_spawn` parent link of old-format subagents | Fix or document |
| `uro-x146` | Claude fork replays decide spawn parents after deletion | Fix or document |
| `uro-s71z` | `--session` misses other sessions’ claims on a shared response | Fix or document |
| `uro-d5g9` | Codex twin pairing with missing-as-zero counts | Fix or document |
| `uro-273d` | Keep the Codex turn context after a context without `turn_id` | Fix or document |
| `uro-px9o` | Count a Codex step when `last_token_usage` is absent | Fix or document |
| `uro-bst2` | Canonicalize gap threads through thread aliases (latent) | Fix or document |

**Decisions and fixture questions**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-xpd0` | How a request observed only as copies is reported | Decide |
| `uro-wt2q` | How observations of a conflicting shared key are partitioned | Decide |
| `uro-c7ro` | Whether an idle source’s unterminated final record is read | Decide |
| `uro-je0v` | Cache-write category for writes with no recorded lifetime | Decide |
| `uro-01xj` | Fixture cases’ open reconciliation questions | Decide |
| `uro-89s7` | Unverified Claude transcript shapes in the fixtures | Decide |
| `uro-mzvt` | `UROLLUP_*` override namespace before the contract freezes | Decide |

**Memory**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-kpup` | Worker line buffers regrow process memory on macOS | Gate |
| `uro-r55i` | Validate `--max-ram` and `--max-rows`; accurate help | Gate |
| `uro-zrr0` | Process-wide admission slices 5–8 (`uro-6pi8`, draft PRs #40 and #41, `uro-2hzh`), density scaling (`uro-z1h1`), representative evidence (`uro-erqo`), Windows job limits (`uro-wt8f`) | Gate, subject to `uro-jzje` |

**Tooling and tests**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-6add` | Supply-chain doc misstates jiff time-zone features | Gate |
| `uro-2zcg` | Honor `CARGO_TARGET_DIR` in parity targets; keep gate proofs out of the repo | Fix or document |
| `uro-x3sk` | e2e naive-sum context line double counts | Fix or document |
| `uro-loml` | Compare tokens per model and effort bucket in e2e results | Fix or document |
| `uro-v1x4` | Look up Claude quota-limit owners by key | Fix or document |

### Phase 2: Supported Costs and Usable CLI Output

Epic `uro-eu9x`. New beads are its children.
Existing pricing beads block it.

**Costs**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-381f` | Retain pricing context and expose cache-request metrics | Gate |
| `uro-wuby` | Reviewed, versioned price table | Gate |
| `uro-neii` | Price matching: dates, tiers, context bands, cache-write durations | Gate |
| `uro-7nhm` | Priced, assumed and unpriced coverage; staleness; `--require-priced` | Gate |
| `uro-i23f` | Golden repricing tests and exact decimal money | Gate |
| `uro-1uy7` | Thread-less Codex settings never set a migrated fork’s tier | Fix or document |
| `uro-unf7` | Show list-price estimates and coverage in report, daily and sessions (after `uro-wuby`, `uro-neii`) | Gate |
| `uro-8aum` | Decode Claude server tool use counts | Fix or document |

**Output and selection**

| Bead | Work | Class |
| --- | --- | --- |
| `uro-tsxr` | Aligned, width-aware tables with totals rows | Gate |
| `uro-eqia` | Sessions view identifies sessions (after `uro-tsxr`) | Gate |
| `uro-f5ss` | Map worktree working directories to their repository’s project | Gate |
| `uro-1uye` | State default selections; explain empty and failed runs | Gate |
| `uro-4u5a` | `--since` and `--until` | Gate, subject to `uro-04vk` |
| `uro-n4f0` | `--agent` | Gate, subject to `uro-04vk` |
| `uro-kxna` | Narrow `--current` discovery to the selected session | Fix or document |
| `uro-bjoc` | Show own and descendant usage separately in report | Fix or document |

### Phase 3: Acceptance and 0.1.0 Release

Epic `uro-0h0g`, blocked by the beads below.
G1 and G2 wait on Phase 1; G3 waits on Phase 2; full-history QA waits on both.

| Bead | Work |
| --- | --- |
| `uro-d36a` | G1: this project’s Claude Code sessions end to end, against pinned ccusage |
| `uro-rm8z` | G2: all recent local sessions, with the Codex archive mounted |
| `uro-o69v` | G3: CLI usability pass |
| `uro-ky6c` | Full-history QA playbook and leftover doc wording |
| `uro-ne7g` | ccusage cost parity (after `uro-unf7`) |
| `uro-2025` | README, CHANGELOG with contract versions, known limitations, runbook |
| `uro-vboa` | 0.1.0 release checklist, packaging rehearsal on the accepted commit, shadow-mode validation; it waits on the packaging beads under `uro-30ef` |

## Testing Strategy

- **Per bead:**
  - Every fix gets a synthetic regression in `urollup-core` tests.
  - Every realistic shape also gets a fixture case with an `expected.json` and its
    transcript golden (`tests/golden/README.md`).
  - Coverage changes assert the reasons and magnitudes, not only `complete: false`.
- **Robustness:**
  - Tests for races, permissions, FIFOs, truncated sidecars, empty IDs and overflowing
    counts.
  - Each asserts exit 0, a gap and a diagnostic, and the other agent’s totals unchanged.
- **Memory:** a macOS footprint regression with many lines over 8 MiB, and the existing
  scale gate.
- **Gate:** `make check` on every PR head, with hosted CI.
- **Real history, local and aggregate-only:**
  1. Rerun the full-history playbook with every default root mounted.
  2. Check cross-report conservation on input frozen for the run.
  3. Compare 1 worker against 8.
  4. Run token and cost parity against pinned ccusage.
  5. Explain each residual in the parity ledger.

## Rollout Plan

PR boundaries follow the code, not the bead tables.
Several beads may share one PR, and none needs a stack unless review asks.

- **Phase 1:**
  - The coverage block and its magnitudes.
  - Discovery and reader robustness.
  - Codex accounting fixes.
  - Claude adapter fixes.
  - The line buffer and `--max-ram` validation.
  - The admission stack already in review (#40, #41, then slices 7–8).
- **Phase 2:**
  - The pricing stack, starting from `feat/pricing-engine`.
  - Table rendering and the sessions view.
  - Project mapping.
  - Selection additions.
- **Phase 3:**
  - Acceptance runs record aggregates locally.
  - The documentation PR.
  - Then the release checklist.

Merges and publication remain maintainer actions under the recorded policy grants.

## Open Questions

- `uro-jzje`: does the alpha keep admission slices 7–8, `uro-z1h1` and `uro-erqo` as
  gates, or tag with the row ceiling and move them to the next pre-1.0 release?
- `uro-04vk`: do `--since`/`--until` and `--agent` join the alpha?
  Do weekly and monthly views stay in 0.5?
- Does the alpha need `--prices` overrides (`uro-640p`), or only the bundled table?
- For colliding project basenames after worktree mapping, should output show a
  disambiguating parent?
- Should the stale stack-landing beads under `uro-rfcr`, whose PRs merged into
  `4c55617`, be closed?

## References

- [Alpha readiness review](../../reviews/review-2026-10-23-alpha-readiness.md)
- [Release readiness record](plan-2026-09-16-first-release-publishing.md#current-readiness-and-critical-path)
- [Core CLI delivery slice](plan-2026-09-20-usage-analysis-workflow.md#core-cli-delivery-slice-2026-09-27)
- [Scalable-ingestion plan](plan-2026-09-16-scalable-ingestion.md)
- [Product plan, milestone 0.1](plan-2026-09-13-urollup-cli-and-web.md#milestone-01-uncached-claude-code-and-codex-reports)
- [Full-history QA playbook](../../../../tests/qa/full-history-rollup.qa.md)
- [Agent format contracts](../../../formats/README.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
