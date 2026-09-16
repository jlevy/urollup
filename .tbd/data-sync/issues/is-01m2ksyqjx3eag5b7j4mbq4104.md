---
type: is
id: is-01m2ksyqjx3eag5b7j4mbq4104
title: "Phase 3: Ledger and query cache"
kind: epic
status: open
priority: 2
version: 6
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-3
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2ksyv7sj854y8xb6gh0sqyg
  - is-01m2f0tk5g8r3bxm5s2r4mfajg
  - is-01m2f0tkfj316rc6apwjpz9sjb
  - is-01m2f0tksf73bnhw9pvzwbe09t
  - is-01m2f0tm43qfyq1hn5s796yc42
created_at: 2026-09-16T00:30:34.575Z
updated_at: 2026-09-16T00:31:39.511Z
---
Phase 3 of the urollup plan: the idempotent persistent cache for capture layers 2 and 3 (normalized tables and usage summaries), urollup's own store as a read-only input, and proof that cached and uncached results match.

Design: §8.3 (Ledger and Query Cache), §5.1 (database input) and Decision 20. Storage is chosen after benchmarks expose access patterns; transactional embedded storage is the design direction, not a dependency decision made in the plan.
