---
type: is
id: is-01m4p720gxeqghmsm4dyz3zj03
title: Charge the coordinating thread's intern caches in admission
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m3gvh6vhs5keehwyxd6r88vs
created_at: 2026-10-11T19:29:26.036Z
updated_at: 2026-10-11T19:29:26.036Z
---
Found in PR #19 review D (D2). The admission model charges the per-thread Name and PricingContext intern caches in each worker slot, which fits Claude (interns on workers). Codex interns on the coordinating thread during normalize, after workers join, and so does the reconcile pricing merge; that thread's caches (up to a few hundred bytes) have no live charge once the coordinating slot is released. Charge a coordinating-thread intern cache term at the checkpoints where the coordinating slot is released (slice 7 wiring), and cover it in the memory_model harness.
