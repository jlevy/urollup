---
type: is
id: is-01m2xm3qa0jxax4tsdx8m93kgc
title: Compact KeyGraph IDs during grouping
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
created_at: 2026-09-19T20:00:51.006Z
updated_at: 2026-09-19T20:01:15.121Z
---
KeyGraph keeps HashMap<AnalyticalId, u32> plus a parallel ids Vec, so each 17 B ID is stored twice through the grouping pass beside every 224 B shell. Compact in reconcile.rs KeyGraph (store each ID once; intern or index-only map). Do not drop the graph after grouping (uro-kvrb). Do not remake boxing, chunked consume, field shrinks, or intern-lifetime observe reorder. Peak must fall.
