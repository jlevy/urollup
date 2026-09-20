---
type: is
id: is-01m2y0s71tysh2p65zrst6wmg3
title: PR leftover landed compact-row work and the scalable-ingestion branch
kind: task
status: in_progress
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
dependencies: []
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
created_at: 2026-09-19T23:42:18.168Z
updated_at: 2026-09-19T23:42:27.448Z
---
The landed Phase 1 cuts (EvidenceRef 16 B, bounded worker line buffers, compact Measures, packed sequence, interned limit rows, once-stored KeyGraph IDs) and Codex-first ingest sit uncommitted on scalable-ingestion. Publish them as their own stacked PR, separate from the RAM-relative ceiling, and open a PR for the existing committed scalable-ingestion engine against the terminal-UX parent. Do not reopen canceled 512 experiments.
