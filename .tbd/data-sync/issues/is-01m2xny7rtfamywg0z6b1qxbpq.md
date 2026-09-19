---
type: is
id: is-01m2xny7rtfamywg0z6b1qxbpq
title: Pack observation keys to KeyGraph nodes
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
created_at: 2026-09-19T20:32:48.409Z
updated_at: 2026-09-19T20:33:00.444Z
---
After uro-t8ws, KeyGraph holds each AnalyticalId once. Each RequestObservation still embeds full DerivedKey IDs (InlineList of two) through grouping. Pack in reconcile.rs resolve_identities / RequestObservation.keys so a registered key stores a node index (plus precedence, basis, check) instead of a second AnalyticalId. Do not drop KeyGraph after grouping (uro-kvrb). Do not shrink to one inline slot (uro-b3gg). Peak must fall.
