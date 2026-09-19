---
type: is
id: is-01m2xny7rtfamywg0z6b1qxbpq
title: Pack observation keys to KeyGraph nodes
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
created_at: 2026-09-19T20:32:48.409Z
updated_at: 2026-09-19T20:54:06.485Z
started_at: 2026-09-19T20:33:31.338Z
closed_at: 2026-09-19T20:54:06.484Z
close_reason: Packing observation keys to KeyGraph nodes did not lower peak. Join-time HashMap intern, join-time KeyGraph, and concat-then-pack at reconcile left quiet whole-history at 682, 657 and 661 MiB versus the uro-t8ws standing 653 MiB (668384 KiB). The IDs are already live in every grouping shell; relocating them adds a second table. Two inline slots kept (uro-b3gg). KeyGraph kept after grouping (uro-kvrb). Reverted. Tests green after restore.
resolution: null
duplicate_of: null
---
After uro-t8ws, KeyGraph holds each AnalyticalId once. Each RequestObservation still embeds full DerivedKey IDs (InlineList of two) through grouping. Pack in reconcile.rs resolve_identities / RequestObservation.keys so a registered key stores a node index (plus precedence, basis, check) instead of a second AnalyticalId. Do not drop KeyGraph after grouping (uro-kvrb). Do not shrink to one inline slot (uro-b3gg). Peak must fall.
