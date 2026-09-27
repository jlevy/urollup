---
type: is
id: is-01m2p3evvpmcn8cf59wc1196f6
title: Scalable whole-history ingestion
kind: feature
status: in_progress
priority: 0
version: 16
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
  - testing
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
child_order_hints:
  - is-01m2pkgts2b87n25929xphbnpc
  - is-01m2pkgv1mh7268dh4sxbptdmg
  - is-01m2pkgva22qd452me3cdjc1fq
  - is-01m2y0s6ebd4a1hrsrhwsvntkh
  - is-01m2y0s71tysh2p65zrst6wmg3
  - is-01m3gvfcvb1b1edkhbsy9v8wb8
  - is-01m3gvh7hg96aprx3v77c4pf6j
created_at: 2026-09-16T21:55:09.300Z
updated_at: 2026-09-27T07:16:39.675Z
---
Whole-history ingestion for Claude Code and Codex must stream raw logs larger than RAM, retain compact usage records, and remain manageable on a developer laptop. The approved 2026-09-27 policy replaces representative 512 MiB/10-second thresholds with density-aware scale evidence and conservative process-wide memory admission. Phase 1 uro-n1cp preserves compact-engine implementation; Phase 2 uro-zrr0 owns release safety and scale acceptance; Phase 3 uro-ky6c owns full-history QA. Keep efficient in-memory reconciliation for 0.1. Dense over-budget inputs must fail early and clearly; hybrid external-memory completion is follow-up uro-924y. Reuse uro-6kwn and uro-xm48 for artifacts/cache work. Private evidence stays local.

## Notes

2026-09-19: Phase 1 field/ID cuts exhausted at WH 653 / Codex-only 586. Phase 2 (uro-zrr0) now owns both 512 MiB and 10 s. uro-n1cp waits on uro-zrr0. G1 waits on uro-zrr0.
