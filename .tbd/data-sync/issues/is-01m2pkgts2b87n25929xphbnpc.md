---
type: is
id: is-01m2pkgts2b87n25929xphbnpc
title: "Scalable ingestion phase 1: whole history works"
kind: task
status: in_progress
priority: 0
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
created_at: 2026-09-17T02:35:50.945Z
updated_at: 2026-09-19T07:40:15.878Z
---
Finish Phase 1 of the scalable-ingestion plan: compact Codex decoded records and the observation/request rows so default whole-history sessions, daily and report --all stay at or below 512 MiB peak footprint on the maintainer corpus. Already landed: the owner-map fix, in-place compact rows, bounded workers, guard removal, the 2 GiB compact-row ceiling, UROLLUP_STATS, the native session field, worker-count identity tests, and whole-history completion in about 18–30 s at about 1.14 GB. Acceptance: make check; whole history exits 0 in at most 25 s at no more than 512 MiB; one and eight workers give identical JSON.

## Notes

2026-09-19: Claimed. The original second-engine / FamilyBatch / projection-oracle checklist was abandoned; the existing engine was compacted in place (see the spec Progress section). Current gap is Codex and row size: default whole history including the archive peaks around 1.14 GB versus the 512 MiB goal.
