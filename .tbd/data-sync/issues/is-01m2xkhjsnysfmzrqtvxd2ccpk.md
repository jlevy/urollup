---
type: is
id: is-01m2xkhjsnysfmzrqtvxd2ccpk
title: Compact Codex limit rows before grouping
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T19:50:56.542Z
updated_at: 2026-09-19T19:51:07.083Z
---
137k ProviderLimitObservation rows stay live through Codex grouping beside every request shell. Each row owns limit_name, window, a Box<str> native JSON, and a Basis<Timestamp>. Compact in entities.rs ProviderLimitObservation and codex_rollout.rs append_limits / normalize so grouping does not hold fat limit strings. Do not re-attempt intern-lifetime observe reorder (uro-mxyh) or KeyGraph drop (uro-kvrb). Peak must fall.
