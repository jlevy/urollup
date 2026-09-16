---
type: is
id: is-01m2kt31tkfrxmavpxhcz96302
title: "Phase 3: Ledger and query cache"
kind: epic
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-3
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2kt32v8spyp96g40c8495dc
created_at: 2026-09-16T00:32:56.146Z
updated_at: 2026-09-16T03:02:07.439Z
closed_at: 2026-09-16T03:02:07.437Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-py01 is the original.
resolution: duplicate
duplicate_of: is-01m2ksyqjx3eag5b7j4mbq4104
---
Phase 3 of the urollup plan: the idempotent persistent cache for capture layers 2 and 3 (normalized tables and usage summaries), urollup's own store as a read-only input, and proof that cached and uncached results match.

Design: §8.3 (Ledger and Query Cache), §5.1 (database input) and Decision 20. Storage is chosen after benchmarks expose access patterns; transactional embedded storage is the design direction, not a dependency decision made in the plan.
