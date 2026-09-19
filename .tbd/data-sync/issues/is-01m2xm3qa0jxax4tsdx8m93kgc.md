---
type: is
id: is-01m2xm3qa0jxax4tsdx8m93kgc
title: Compact KeyGraph IDs during grouping
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
created_at: 2026-09-19T20:00:51.006Z
updated_at: 2026-09-19T20:33:00.142Z
started_at: 2026-09-19T20:02:05.939Z
closed_at: 2026-09-19T20:33:00.141Z
close_reason: KeyGraph stores each AnalyticalId once in ids and looks up through open-addressed u32 slots. Tests green. Quiet WH 653 MiB (668384 KiB) / 17.3 s (repeat 661); Codex-only 586 MiB (600336 KiB) / 13.2 s. Peak fell versus uro-4h93 670/583. Gate still 512.
resolution: null
duplicate_of: null
---
KeyGraph keeps HashMap<AnalyticalId, u32> plus a parallel ids Vec, so each 17 B ID is stored twice through the grouping pass beside every 224 B shell. Compact in reconcile.rs KeyGraph (store each ID once; intern or index-only map). Do not drop the graph after grouping (uro-kvrb). Do not remake boxing, chunked consume, field shrinks, or intern-lifetime observe reorder. Peak must fall.
