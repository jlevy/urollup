---
type: is
id: is-01m2pkgva22qd452me3cdjc1fq
title: "Scalable ingestion phase 3: full-history QA and cleanup"
kind: task
status: open
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - qa
dependencies: []
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
child_order_hints:
  - is-01m2y5dtedbdwka9v6mwcdkass
created_at: 2026-09-17T02:35:51.489Z
updated_at: 2026-09-27T07:26:14.359Z
---
Finish Phase 3 after revised scale gate uro-zrr0 and project G1 uro-d36a: execute tests/qa/full-history-rollup.qa.md on the maintainer machine, retain a dated QA report outside disposable scratch, and publish only separately authorized evidence. Update leftover design, plan and README wording. The one-pass join in tests/parity/local_diff.py already landed. The engine was compacted in place; there is no second engine or projection oracle to delete. The 2026-09-27 policy replaces fixed representative 512 MiB/10-second limits, but accounting, parity, invariants and worker determinism remain required.

## Notes

2026-09-19: Parity join done. Blocked on uro-zrr0 and uro-d36a so Phase 2 and G1 land before the full-history playbook.
