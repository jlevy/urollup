---
type: is
id: is-01m2pkgts2b87n25929xphbnpc
title: "Scalable ingestion phase 1: whole history works"
kind: task
status: open
priority: 0
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
created_at: 2026-09-17T02:35:50.945Z
updated_at: 2026-09-17T02:36:08.507Z
---
Compact ledger, streaming family decode and bounded workers so whole-history sessions, daily and report --all work on the full local corpus. Includes the owner-map ordering fix, projection oracle, compact rows with size assertions, streaming limit collapse, per-code diagnostics, guard removal with a 2 GiB compact-row ceiling, UROLLUP_JOBS and UROLLUP_STATS, and the native session field on sessions rows. Acceptance: make check passes; whole history exits 0 in at most 25 s at no more than 512 MiB peak footprint; one and eight workers give identical JSON.
