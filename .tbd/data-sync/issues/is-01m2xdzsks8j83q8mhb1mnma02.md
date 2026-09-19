---
type: is
id: is-01m2xdzsks8j83q8mhb1mnma02
title: Reserve Codex observations per rollout after worker results drop
kind: task
status: closed
priority: 0
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
hold: null
hold_until: null
created_at: 2026-09-19T18:13:50.831Z
updated_at: 2026-09-19T18:17:36.029Z
started_at: 2026-09-19T18:13:58.326Z
closed_at: 2026-09-19T18:17:36.027Z
close_reason: "Landed: codex_rollout.rs normalize reserves and appends observations per rollout instead of with_capacity(sum) beside every worker result. Fixture adapters/snapshots/worker-count passed. Codex-only sessions --all: 650 MiB (665904 KiB) / 14.0 s, down from 760 MiB. Whole history: 647 MiB (662576 KiB) / 34.7 s, 770 MiB / 19.0 s, one 855 MiB loaded outlier. Same row counts. Uncommitted. Still above 512 MiB."
resolution: null
duplicate_of: null
---
Dialect-split sessions --all: Codex-only 760 MiB (760224 KiB), Claude-only 357 MiB (365696 KiB). Whole-history peak is Codex ingest. normalize does Vec::with_capacity(sum of every rollout observation slot) while try_read_in_parallel still holds every DecodedRollout.Observed.observations (612,561 rows at 264 B is about 154 MiB reserved beside the same rows). Reserve and append per rollout after earlier worker vectors move. Files: codex_rollout.rs normalize. Peak must fall.
