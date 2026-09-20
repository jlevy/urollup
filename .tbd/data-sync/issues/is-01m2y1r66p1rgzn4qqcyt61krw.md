---
type: is
id: is-01m2y1r66p1rgzn4qqcyt61krw
title: Add Cursor agent and provider grouping facets
kind: task
status: in_progress
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: unknown@spud10
labels: []
dependencies:
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-09-19T23:59:13.108Z
updated_at: 2026-09-20T07:01:46.116Z
started_at: 2026-09-20T07:01:46.113Z
---
Phase 1 facets from plan-2026-09-19-cursor-dialect.md. Add Agent::Cursor (token cursor) through selection, discovery, CLI --agent, and QuerySource. Add request-level provider (Basis plus vendor registry token) and --group-by provider. Add --group-by agent if still missing.

Do not use cursor as a model value or as the provider token. Account stays unknown unless recorded. No dialect adapter in this bead.
