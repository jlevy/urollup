---
type: is
id: is-01m3gvh7hg96aprx3v77c4pf6j
title: Design hybrid spill for histories whose retained state exceeds the memory budget
kind: feature
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - follow-up
  - memory
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-27T07:16:07.855Z
updated_at: 2026-09-27T07:36:33.311Z
---
Follow-up after 0.1, not a release blocker. Preserve the measured fast in-memory path and spill compact partitioned or sorted runs only as the process-wide budget is approached. Complete dense histories beyond RAM while preserving global Claude ownership, Codex lineage, identity/collision checks, accounting, exact percentiles and deterministic ordering across partition boundaries. Specify bounded merge fan-in, scratch/disk budgeting, permissions, cancellation, cleanup and disk-full behavior; test cross-partition keys, long families, output equivalence and failure recovery. Do not claim this is implemented. Coordinate reusable normalized artifacts and incremental cache maintenance with existing uro-6kwn and uro-xm48 instead of creating another accounting engine.
