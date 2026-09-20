---
type: is
id: is-01m2y1rag1z513ncz2mr4k7vk9
title: Implement Cursor discovery and usage adapter
kind: task
status: in_progress
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: unknown@spud10
labels: []
dependencies:
  - type: blocks
    target: is-01m2y1redkntka5x0rwr6kjmkb
  - type: blocks
    target: is-01m2y1rhynxj8zrnp4368g534e
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-09-19T23:59:17.502Z
updated_at: 2026-09-20T07:01:46.146Z
started_at: 2026-09-20T07:01:46.146Z
---
Phase 1 adapter from plan-2026-09-19-cursor-dialect.md. Default discovery (transcripts and application user state), UROLLUP_CURSOR_DIRS, snapshot manifests, strip policies, identities, copy/resume/subagent/Best-of-N rules, and coverage gaps. Count only recorded usage fields.

Named Cursor state-store reader is in scope; generic SQLite input (Decision 20) is not. Do not invent record shapes; use the locked brief. Do not change ingest-capacity or scalable-ingestion code.
