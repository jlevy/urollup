---
type: is
id: is-01m2y4ftbwp0dedvfmtpqyx5tq
title: Audit published PR stack and memory-efficiency acceptance gaps
kind: task
status: closed
priority: 1
version: 10
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
child_order_hints:
  - is-01m2y4ydxjz5xpq81vf3ecd9rv
  - is-01m2y4yf4mejx5e6b80an6f3w4
  - is-01m2y4yge5wcpbvpnp7ye70ht1
  - is-01m2y4zmb44cb9crzfq7ezf1p0
  - is-01m2y4zp4nvy4ggpg0y94dzrbe
  - is-01m2y50zntpg0bavafh8547sfb
created_at: 2026-09-20T00:47:04.571Z
updated_at: 2026-09-20T01:05:03.728Z
closed_at: 2026-09-20T01:05:03.727Z
close_reason: "Audit complete: exact-head CI and review inventory, fresh local make check passes, five findings reproduced/traced and tracked, three full-review passes mapped, stack unified, PR descriptions updated. Implementation defects, Windows CI and 0.1 acceptance remain open under their owning beads."
resolution: null
duplicate_of: null
---
Validate current PR heads, review dispositions, CI, memory implementation and evidence; record a merge-order and remaining-work handoff without merging PRs.

## Notes

---
title: PR Stack and Memory Audit
description: Published-head validation, reproduced findings, merge order, and the remaining 0.1 acceptance work.
date: 2026-09-19
status: Audit complete; implementation and milestone acceptance blocked
---
# PR Stack and Memory Audit

The implementation stack is organized, but it is not ready for unconditional merge
approval. This audit found five actionable issues, two with reproduced accounting or
evidence regressions.
The whole-history memory and speed targets remain unmet.
Audit evidence is also recorded in bead `uro-y7zm`; implementation findings remain open.

## Published Heads and Review Coverage

| PR | Head | Base | CI snapshot | Review verdict |
| --- | --- | --- | --- | --- |
| [#4](https://github.com/jlevy/urollup/pull/4) | `c08986d` | `main` | 13 checks passed | Full implementation review remains open |
| [#8](https://github.com/jlevy/urollup/pull/8) | `efa5ef8` | `milestone-0.1` | 13 checks passed | Full terminal, privacy, and source-path review remains open |
| [#10](https://github.com/jlevy/urollup/pull/10) | `827dad2` | `codex/v0.1-terminal-ux-acceptance` | 13 checks passed | Changes required: early capacity enforcement and CI scale wiring |
| [#11](https://github.com/jlevy/urollup/pull/11) | `d44476b` | `scalable-ingestion` | 13 checks passed | Changes required: source evidence and sequence boundaries |
| [#12](https://github.com/jlevy/urollup/pull/12) | `d446732` | `ingest-compact` | 12 passed; Windows goldens still running | Changes required: RAM-probe behavior and complete Windows validation |
| [#14](https://github.com/jlevy/urollup/pull/14) | `6c3fc36` | `main` | No checks; local formatting passed | Separate docs review and integration follow-up remain |

GitHub had zero formal reviews and zero inline review threads on all six PRs at the
start of the audit. PR #14 had one CI-retrigger comment.
No earlier independent full implementation review was located in the repository or
tracking record.

This audit inspected the current PR descriptions, bases, exact heads, review history, CI
jobs, memory-related diffs and their surrounding code, and recorded benchmark and
acceptance evidence.
It traced capacity enforcement, worker-result retention, source-index remapping, compact
token measures, sequence packing, interned names, identity grouping, and the scale
harness. It ran synthetic reproductions and the local handoff gate.
It is a focused audit, not a line-by-line review of the entire 297-file base
implementation and all subsequent layers.
`uro-nncx` remains open for that work.

## Findings

### R1: Capacity enforcement occurs after unbounded decode retention

**High; `uro-y1lp`; introduced in #10, exposed as a RAM budget in #12.** Both adapters
collect all worker results before normalization.
The only observation ceiling check is at
`crates/urollup-core/src/ledger/reconcile.rs:505`, after the full observation input
already exists. The relevant callers are `adapters/claude_project.rs:550` and
`adapters/codex_rollout.rs:579`.

A 32 MiB, seed-1 synthetic corpus contains 21,253 usage records.
A release report with `--max-rows 1` still constructed 19,757 Codex observations before
refusing.
It reached 35.44 MiB peak physical footprint and 37.14 MiB maximum RSS, exiting
1 after 0.73 s. This is a small, bounded reproduction of the late check, not a claim
that a one-row budget should cover the process’s fixed startup cost.
Larger inputs can exhaust memory before the intended diagnostic occurs.
The row-shell calculation also excludes payloads, intern tables, requests, other ledger
tables, and the other agent’s retained ledger.

**Fix:** enforce a shared per-agent budget before retaining decoded observations,
including pending representations, and propagate cancellation and capacity errors.
Document precisely that a row budget is not a process RSS limit.
Test early refusal, worker-count behavior, and unchanged successful accounting.

### R2: Claude malformed-record references retain source index zero

**High; `uro-ww39`; #11.** The reader now emits local `EvidenceRef.source = 0`. Claude
normalization builds a global source table but does not remap
`manifest.entries[].first_malformed` before cloning those snapshots into source
artifacts (`crates/urollup-core/src/adapters/claude_project.rs:980`, `:1030`). Codex
performs this remap at `codex_rollout.rs:1165`.

Two synthetic Claude files, each with a valid record and malformed second line, produced
only **1/2** malformed references resolving to the correct source ID. The bug corrupts
provenance; it does not change the aggregate malformed-record count.

**Fix:** stamp these references through the frozen source table before copying the
manifest. Test both manifest entries and source artifacts across multiple sources and
worker counts.

### R3: Packed sequence loses the maximum accepted integer

**Medium; `uro-u6in`; #11.** `NativeSequence::new` at
`crates/urollup-core/src/ledger/reconcile.rs:68` returns `None` for `u64::MAX`. The
Claude adapter silently uses that result at `adapters/claude_project.rs:1397`, although
the typed decoder accepts that integer.

For two revisions of one synthetic request with equal output of 10, block indices
`[18446744073709551615, 1]`, and input counts `[100, 1]`, the report selects 1 input
token and totals 11. The documented largest-block-index tie-break selects 100 input
tokens and totals 110. This is a boundary regression, not evidence that ordinary
provider logs use such a large block index.

**Fix:** preserve the accepted domain and ordering, or explicitly diagnose a narrower
supported dialect domain.
Test missing, zero, maximum-minus-one, maximum, reversed arrival order, and final
totals.

### R4: GitHub CI does not execute the synthetic scale workload

**High; `uro-a8fk`; #10.** `Makefile:55` includes `scale-gate` in `make test`, but
`.github/workflows/ci.yml` calls the individual Rust, QA-tool, golden and result
commands. No workflow invokes `make test`, `make check`, `make scale-gate`, or
`scripts/check-scale.py`. QA unit tests exercise the scale gate’s logic, and the
failing-test probe exits before reaching the scale workload.
Neither measures the release engine in CI.

**Fix:** add an explicit POSIX release scale step behind the supply-chain gate and
protect that wiring with a regression check.
Record its measured results.
Keep the small synthetic regression gate distinct from whole-history acceptance.

### R5: RAM detection can block even when no RAM query is needed

**High; `uro-qvy0`; #12.** `crates/urollup/src/cli.rs:370` always evaluates
`physical_memory_bytes()`, including when an exact `--max-rows` or byte budget was
supplied. Both platform helpers use unbounded `Command::output`
(`ledger/capacity.rs:309`, `:318`). A synthetic three-second `sysctl` delayed an
explicit `--max-rows 1` invocation to 4.16 seconds.
A stuck helper can prevent the fallback from ever being reached.

Separately,
[the Windows CI job](https://github.com/jlevy/urollup/actions/runs/35478104257/job/105990831189)
has remained in CLI goldens since 2026-09-20 00:13:45 UTC. Its Rust tests passed, but
goldens and the following fixture-result step have not completed.
GitHub does not yet provide its job log.
PowerShell is a hypothesis for that stall, not a confirmed cause.

**Fix:** resolve explicit budgets without probing the host, bound and reap any helper or
use a reviewed native facility, preserve read-only HOME behavior, and obtain a completed
green Windows run. Add tests with injected slow and failed probes.

## What the Existing Evidence Establishes

- Fresh `make check` on integrated head `d446732` exited 0, including all 30 negative
  gate proofs, Rust 1.98 default/no-default tests, Rust 1.85 MSRV tests,
  format/lint/docs, fixture/golden/parity checks, supply-chain/dependency audits and the
  local scale gate. This is a local macOS result; the outstanding Windows job remains
  separate.
- Current fixture validation covers 29 cases; the CLI goldens cover 30 sessions and 239
  blocks. Pinned ccusage parity covers 290 comparisons and 403 explained differences.
- Worker-count tests compare one worker with 2, 3, 8 and 64 workers, both per fixture
  and with each dialect’s fixtures combined.
  They also check first-error selection.
- Typed Claude and Codex parser tests compare against document-parsing oracles, with
  2,000 generated cases each.
  Compact measures retain values beyond `u32`, and tests exercise their value, ordering
  and hashing behavior.
- `EvidenceRef` uses explicit source/offset/length order, independent of field layout.
  Its 16-byte representation and 224-byte observation rows have compile-time guards.
- The local scale run passed: padded-content peak difference 1.91 MiB, fitted slope
  2,565 B/usage record with a 3.45 MiB intercept, and a 32 MiB daily corpus containing
  21,253 usage records under the watchdog at 52.7 MiB sampled RSS.

These checks are useful regression evidence.
They do not prove the maintainer corpus fits within 512 MiB, nor that all code has
received independent review.

The historical standing whole-history measurement is **653 MiB / 17.3 s**, with a repeat
at 661 MiB / 18.6 s; Codex-only is **586 MiB / 13.2 s**. A later quiet profile recorded
648 MiB / 18.2 s and 573 MiB / 14.3 s. These are recorded earlier measurements, not new
measurements of this audit’s head.
The whole-history gap is roughly 141 MiB and 7–8 seconds.
No private logs were read for this audit.

## Design Assessment and Remaining Limits

Compact in-place rows and one global reconciliation preserve Claude’s cross-session
ownership semantics.
Per-family reconciliation is not an interchangeable optimization.
The row layout work has meaningful invariance tests, but R2 and R3 show why end-to-end
boundary cases must accompany representation changes.

Process-global name and overflow-measure intern tables deliberately retain distinct
values until process exit.
Provider-limit JSON can have high cardinality.
This is a material constraint for repeated library ingestion or a future server; small
row sizes do not establish bounded lifetime or total memory.
Scope or measure those tables before claiming support for a long-lived process.
Linux RAM detection reads host `MemTotal`, not a container memory limit.

The 512 MiB target should be pursued from measurements of live allocation ownership and
representative record density.
Do not repeat the reverted field/ID relocation, primitive-number, mimalloc,
zero-copy-acceptance, or larger-read-window experiments.
`uro-nzo1` is held. `uro-lsaz` owns profiling and `uro-a3fo` the remaining sidecar work;
neither has evidence yet that it will close the full gap.

## Remaining Work and Merge Order

1. Fix R1–R5 on their owning layers, then propagate each fix upward and obtain fresh
   per-head CI. Preserve synthetic regressions for the reproduced failures.
2. Complete `uro-nncx`: accounting/identity/reconciliation (`uro-syzt`),
   readers/adapters/selection/CLI (`uro-jw7u`), and harness/CI/privacy/portability plus
   final integration (`uro-dhek`). Record exact-head verdicts and every disposition.
3. Complete `uro-zrr0`: prove 512 MiB and 10 seconds with representative whole-history
   measurements and working CI scale gates.
   `uro-n1cp` waits on this result.
4. Run consented G1 (`uro-d36a`) and the full-history playbook (`uro-ky6c`), recording
   aggregates only. Confirm one/eight-worker equality and explain parity residuals.
5. Resolve or explicitly defer the six maintainer decisions (`uro-mzvt`, `uro-wt2q`,
   `uro-je0v`, `uro-xpd0`, `uro-c7ro`, `uro-01xj`) and unverified Claude shapes
   (`uro-89s7`). These are milestone decisions, not silent scope reductions.
6. Merge the reviewed implementation bottom to top: **#4 → #8 → #10 → #11 → #12**. The
   two remote stack groups have been consolidated into stack **#9**, and local metadata
   matches all five heads.
   No branch needs a rebase at the audit snapshot.
   No PR was merged and no implementation commit changed in this audit.
7. Rebase separate Cursor docs #14 after the implementation stack lands (`uro-knnz`). A
   merge simulation found a product-plan conflict: retain both the End-to-End Acceptance
   Goals section and the later-Cursor pointer.
   Its links to implementation code and the scalable-ingestion spec are absent on
   today’s main. All four changed documents pass the pinned local formatter; current main
   has no workflow. Check links, review the research claims and scope, and obtain docs CI
   after rebasing.
8. After milestone acceptance, complete packaging under `uro-30ef`: native builds and
   smoke tests (`uro-uj8w`), publishing workflow (`uro-080d`), provenance/version checks
   (`uro-co0m`), release docs (`uro-2025`), and rehearsal/publication/installation
   validation (`uro-vboa`). Cursor implementation remains outside 0.1.

Stale next-step notes on the milestone, performance and review umbrellas were updated.
The old instructions to redo completed Phase 1 cuts or treat ingestion as unpublished
must not drive the next session.
The specifications still need a final consistency pass after fixes (`uro-erqo`),
including outdated 2 GiB wording in scale scripts, the original-design versus as-built
memory estimates, and the unimplemented CI scale claim.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
