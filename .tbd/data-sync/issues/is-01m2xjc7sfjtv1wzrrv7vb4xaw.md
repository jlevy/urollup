---
type: is
id: is-01m2xjc7sfjtv1wzrrv7vb4xaw
title: Drop Codex intern tables after each rollout is observed
kind: task
status: closed
priority: 1
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
created_at: 2026-09-19T19:30:32.878Z
updated_at: 2026-09-19T19:50:54.670Z
started_at: 2026-09-19T19:30:37.969Z
closed_at: 2026-09-19T19:50:54.656Z
close_reason: Reverted. Interner HashMaps already drop in Interner::finish before observe. Observing all Pending first while holding every Observed shell raised Codex-only to 774-836 MiB (792832-855760 KiB) and WH to 799-857 MiB (817968-878176 KiB) vs standing 614/768. Tests stayed green; row counts unchanged. Peak rose.
resolution: canceled
duplicate_of: null
---
Codex Interner HashMaps are dropped at decode finish, but Pending ParsedSource.strings (and remaining intern maps) stay in the join vec through normalize. Consuming Observed shells first then observing Pending last holds every shell beside every still-pending intern table. Observe-and-drop each Pending source (observe_to_observed) before the combined observation vec is built, and drop known_turns before reconcile. Do not drop KeyGraph after grouping (uro-kvrb). Peak must fall.
