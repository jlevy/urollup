---
type: is
id: is-01m2y1rhynxj8zrnp4368g534e
title: Add Cursor current-session detection
kind: task
status: in_progress
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: cursor@spud10
labels: []
dependencies: []
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-09-19T23:59:25.139Z
updated_at: 2026-09-20T23:16:42.682Z
started_at: 2026-09-20T09:51:23.051Z
---
Map the research brief environment or hook signals onto CurrentEnvironment and Agent::Cursor. Until then, a detected Cursor session exits 2 with an unsupported-dialect diagnostic, as Pi does in 0.1. The only heuristic remains --latest and is never implicit.

## Notes

Cursor --session now resolves agent-transcripts/<uuid>/<uuid>.jsonl to composerId without ingesting JSONL as usage. Shared state.vscdb / cursor-state.json paths are not session selectors. Still uncommitted.
