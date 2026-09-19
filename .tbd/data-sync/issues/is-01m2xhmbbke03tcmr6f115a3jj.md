---
type: is
id: is-01m2xhmbbke03tcmr6f115a3jj
title: Drop unused key-spill pointer from observation shells
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
created_at: 2026-09-19T19:17:30.098Z
updated_at: 2026-09-19T19:28:03.158Z
started_at: 2026-09-19T19:18:58.975Z
closed_at: 2026-09-19T19:28:03.157Z
close_reason: "Reverted. Two-slot ObservationKeys (56 B, interned 3+ tail) compiled and tests stayed green (shell 224 to 216). Peak did not fall: paired A/B WH 805072 KiB vs 24ua 750480 KiB at the same load; other o5c0 WH samples 769680/777168 vs standing 24ua 786656. Codex 589824-704160 vs 24ua 628608-707216 is watchdog noise around an 8 B save. Field shrinks of this size are below the RSS floor and cannot close the 256 MiB gap."
resolution: canceled
duplicate_of: null
---
RequestObservation.keys is still 64 B at grouping (two 27 B Option<DerivedKey> plus an 8 B spill pointer). Do not drop to one inline slot (uro-b3gg). Keep two inline keys and intern overflow for 3+ keys so the common path has no per-row malloc. Peak must fall.
