---
type: is
id: is-01m2ewderzabst5nx5bh3prg29
title: "Research: map common urollup workflows"
kind: task
status: closed
priority: 1
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
created_at: 2026-09-14T02:37:22.077Z
updated_at: 2026-09-14T03:12:29.943Z
started_at: 2026-09-14T02:38:24.262Z
closed_at: 2026-09-14T03:12:29.938Z
close_reason: "Rewrote research Scope and added Common Workflows: current session, selected multi-session, disk-wide inventory, session hierarchy, and a selection-mechanism comparison."
resolution: null
duplicate_of: null
---
Add a Common workflows section to the research brief. (1) Current-session rollup, the most common case: scope is the current session, which urollup auto-detects inside Claude Code and Codex (environment, hook input, cwd plus most-recent transcript, explicit override); document what each platform exposes, verified from public sources. (2) Multi-session rollups for Claude, Codex or both, selected by date range, explicit session list, platform, project and other explicit arguments; the broadest form scans disk for all usage across all dates and sessions. (3) Session hierarchy: map each session to its subagent sessions by crawling native linkage, building an in-memory tree. (4) Output: every workflow writes the same summary format, per session or rolled up, and any number of session summaries aggregate into an aggregate summary.
