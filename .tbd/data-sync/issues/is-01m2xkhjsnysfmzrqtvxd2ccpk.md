---
type: is
id: is-01m2xkhjsnysfmzrqtvxd2ccpk
title: Compact Codex limit rows before grouping
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
created_at: 2026-09-19T19:50:56.542Z
updated_at: 2026-09-19T20:01:14.762Z
started_at: 2026-09-19T19:52:21.045Z
closed_at: 2026-09-19T20:01:14.761Z
close_reason: Interned ProviderLimitObservation names, windows and native JSON as Name. Row ≤128 B. Tests green. Quiet Codex-only 583 MiB (596784 KiB) / 14.3 s; WH 670 MiB (686192 KiB) / 18.0 s (repeat 668.5). Peak fell versus uro-24ua 614/768. Gate still 512.
resolution: null
duplicate_of: null
---
137k ProviderLimitObservation rows stay live through Codex grouping beside every request shell. Each row owns limit_name, window, a Box<str> native JSON, and a Basis<Timestamp>. Compact in entities.rs ProviderLimitObservation and codex_rollout.rs append_limits / normalize so grouping does not hold fat limit strings. Do not re-attempt intern-lifetime observe reorder (uro-mxyh) or KeyGraph drop (uro-kvrb). Peak must fall.
