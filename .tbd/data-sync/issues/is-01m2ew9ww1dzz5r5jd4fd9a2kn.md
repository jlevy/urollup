---
type: is
id: is-01m2ew9ww1dzz5r5jd4fd9a2kn
title: "Plan: define the source of resource observations or defer them"
kind: task
status: closed
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe0xffh11dgc6acjznk3r
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:25.433Z
updated_at: 2026-09-14T02:49:52.700Z
started_at: 2026-09-14T02:38:24.164Z
closed_at: 2026-09-14T02:49:52.699Z
close_reason: Resource collection deferred to a non-goal with a reserved contract; values stay unknown, never zero; provider charges split into their own ledger entity.
resolution: null
duplicate_of: null
---
The ledger includes CPU seconds, RSS, I/O and network, and Phase 2 joins 'resource receipts', but agent logs do not record these and no source is named. Done when the plan names concrete resource data sources and their adapter contract, or moves resource observations to future work while keeping unknown-value semantics.
