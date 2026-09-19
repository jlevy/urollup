---
type: is
id: is-01m2xhmbbke03tcmr6f115a3jj
title: Drop unused key-spill pointer from observation shells
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
created_at: 2026-09-19T19:17:30.098Z
updated_at: 2026-09-19T19:17:33.528Z
---
RequestObservation.keys is still 64 B at grouping (two 27 B Option<DerivedKey> plus an 8 B spill pointer). Do not drop to one inline slot (uro-b3gg). Keep two inline keys and intern overflow for 3+ keys so the common path has no per-row malloc. Peak must fall.
