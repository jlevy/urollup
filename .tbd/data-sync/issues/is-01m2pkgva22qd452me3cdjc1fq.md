---
type: is
id: is-01m2pkgva22qd452me3cdjc1fq
title: "Scalable ingestion phase 3: full-history QA and cleanup"
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - qa
dependencies: []
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
child_order_hints:
  - is-01m2y5dtedbdwka9v6mwcdkass
created_at: 2026-09-17T02:35:51.489Z
updated_at: 2026-09-20T01:03:27.691Z
---
Finish Phase 3 of the scalable-ingestion plan: execute tests/qa/full-history-rollup.qa.md on the maintainer machine, commit a privacy-safe dated QA report, and update any leftover design, plan and README wording. The one-pass session join in tests/parity/local_diff.py already landed. There is no second engine or projection oracle to delete; the engine was compacted in place.

## Notes

2026-09-19: Parity join done. Blocked on uro-zrr0 and uro-d36a so Phase 2 and G1 land before the full-history playbook.
