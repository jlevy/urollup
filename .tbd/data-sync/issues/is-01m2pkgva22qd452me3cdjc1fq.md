---
type: is
id: is-01m2pkgva22qd452me3cdjc1fq
title: "Scalable ingestion phase 3: full-history QA and cleanup"
kind: task
status: open
priority: 1
version: 7
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - qa
dependencies:
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
child_order_hints:
  - is-01m2y5dtedbdwka9v6mwcdkass
created_at: 2026-09-17T02:35:51.489Z
updated_at: 2026-09-29T00:41:24.496Z
---
Finish Phase 3 after revised scale gate uro-zrr0 and project G1 uro-d36a: execute tests/qa/full-history-rollup.qa.md on the maintainer machine, retain a dated QA report outside disposable scratch, and publish only separately authorized evidence. Update leftover design, plan and README wording. The one-pass join in tests/parity/local_diff.py already landed. The engine was compacted in place; there is no second engine or projection oracle to delete. The 2026-09-27 policy replaces fixed representative 512 MiB/10-second limits, but accounting, parity, invariants and worker determinism remain required.

## Notes

Exploratory maintainer-requested run completed on the current dirty worktree above 3c4e9cd, built release mode. Existing whole-history report, sessions, daily, Claude-only report, Codex-only report and current-session descendants report all exited successfully under an external RSS watchdog. Local validation found no disjoint-token arithmetic, cache subtotal or within-report breakdown conservation failures. Private JSON, timing/RSS and validation evidence retained outside the repository; no private values copied here. Coverage remains incomplete. Separate commands read live roots, so cross-command equality, frozen-input determinism, external parity and alpha acceptance remain pending. Agent-specific model/project views work; arbitrary joint pivots and CLI costs remain unimplemented. No new features or product-code edits in this run. Follow-up: investigate unknown Codex model attribution and extreme per-request input-size outlier before accepting those facets; use existing metadata and G1 accounting QA work.
