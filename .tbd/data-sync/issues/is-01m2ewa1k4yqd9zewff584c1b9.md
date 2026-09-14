---
type: is
id: is-01m2ewa1k4yqd9zewff584c1b9
title: "Research: add per-dialect log format survey"
kind: task
status: closed
priority: 2
version: 5
spec_path: docs/project/research/research-2026-09-13-portable-agent-usage.md
delegate: claude-code@spud10.local
labels:
  - research
dependencies:
  - type: blocks
    target: is-01m2ewe3whtebs3z0pgza28pj4
  - type: blocks
    target: is-01m2ewdgre68gzmjq82v8kevhv
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:30.275Z
updated_at: 2026-09-14T03:12:29.184Z
started_at: 2026-09-14T02:38:24.273Z
closed_at: 2026-09-14T03:12:29.183Z
close_reason: Added Log dialects and session linkage subsection with per-dialect layout, usage, counter and linkage tables for claude-project, claude-stream, codex-rollout, codex-exec, pi-session and pi-events, pinned sources, and unverified items marked.
resolution: null
duplicate_of: null
---
The research brief argues accounting distinctions abstractly but never documents which fields each log dialect actually records: request/response IDs, usage fields and cache categories, cumulative versus per-request counters, model/effort, subagent and fork linkage, file layout. Done when a Findings subsection summarizes each Claude, Codex and Pi dialect from public sources (pinned links), with field names only and no private session content, and notes what is unverified.
