---
type: is
id: is-01m2ew9t8d4nxbam4k9se4z4cm
title: "Plan: define or defer database snapshot input"
kind: task
status: closed
priority: 1
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
created_at: 2026-09-14T02:35:22.764Z
updated_at: 2026-09-14T03:00:32.083Z
started_at: 2026-09-14T02:38:24.248Z
closed_at: 2026-09-14T03:00:32.082Z
close_reason: Database snapshot input deferred to Phase 3 when urollup's own store exists; export --database example removed.
resolution: null
duplicate_of: null
---
Phase 1 includes raw/bundle/database input adapters and the CLI example 'export --database ./usage.db', but no concrete external database is named and 'our own store format' does not exist until the Phase 3 cache. Done when the plan either names the specific database(s) supported in Phase 1 with rationale, or moves database input to the phase where a store exists, with the CLI example and Phase checklist made consistent.
