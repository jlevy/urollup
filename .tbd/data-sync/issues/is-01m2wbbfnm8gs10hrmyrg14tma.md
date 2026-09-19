---
type: is
id: is-01m2wbbfnm8gs10hrmyrg14tma
title: Measure whole-history RSS after each 512 MiB compaction
kind: task
status: open
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2pkgts2b87n25929xphbnpc
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T08:08:33.712Z
updated_at: 2026-09-19T16:29:13.390Z
---
After each Phase 1 compaction, run privacy-safe whole-history sessions --all (and daily/report --all when the peak changes) on the maintainer corpus. Record peak RSS, wall time, UROLLUP_STATS row counts and worker-count identity. Quote no paths, IDs or prompt text. Update the scalable-ingestion spec Progress table and the 512 MiB checkbox when the goal is met.

Acceptance: dated privacy-safe numbers in the spec; uro-n1cp closable only when peak is at or below 512 MiB and make check is green.

## Notes

2026-09-19: After uro-ecol, release sessions --all: exit 0, 861 MiB peak (881296 KiB), 22.3 s, workers=8. Same row counts as after uro-brar (Codex 612561 obs / 611314 req, Claude 413742 / 191476). Still above 512 MiB. Next memory cut is typed rate_limits decode (uro-zrr0).
