---
type: is
id: is-01m2p3evvpmcn8cf59wc1196f6
title: Scalable whole-history ingestion
kind: feature
status: in_progress
priority: 0
version: 8
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
created_at: 2026-09-16T21:55:09.300Z
updated_at: 2026-09-19T08:15:34.811Z
---
Replace the temporary v0.1 input-size crash barrier with bounded parallel decode into compact rows so default and --all reports cover multi-gigabyte Claude Code and Codex history. Spill to disk is a non-goal. Children: uro-n1cp (Phase 1, 512 MiB), uro-zrr0 (Phase 2, typed decode and 10 s), uro-ky6c (Phase 3, full-history QA). Distinct from session-family preselection, which already keeps --current and exact selections from reading unrelated logs.

## Notes

2026-09-19: Phase 1 children under uro-n1cp. Dual-Ingested hold removed; whole-history peak 936 MiB. Still short of 512 MiB. Local scalable-ingestion branch unpushed.
