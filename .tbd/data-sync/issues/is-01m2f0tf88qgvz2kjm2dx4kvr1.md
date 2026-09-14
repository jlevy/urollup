---
type: is
id: is-01m2f0tf88qgvz2kjm2dx4kvr1
title: Implement session selection, current-session detection and hierarchy crawler
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tg7kbpmm11wycct2vb5n
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:22.855Z
updated_at: 2026-09-14T19:58:18.655Z
---
Selection flags, --current detection (--hook-input, agent environment variables, else exit 2), guarded --latest, discovery index, and the crawler behind tree and --scope descendants. See plan: Workflows and session selection.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Codex hook input: check the rollout by session_meta.session_id == session_id and, when agent_id is present, session_meta.id == agent_id; on SubagentStop use agent_transcript_path; hooks get no CODEX_* variables; detect nested agents by comparing hook session_id with any CODEX_SESSION_ID in the environment; a null transcript_path is an unsaved session (exit 1).
- Pi: PI_SESSION_ID without PI_SESSION_FILE is unsaved (exit 1); Pi sessions include the calling request.
- For --current, resolve the transcript and its subagents/ directory first and parse only those (ccusage statusline full scans show the cost otherwise).
- Codex SQLite state_*.sqlite threads and thread_spawn_edges as an optional, version-gated discovery hint, never a usage source.
- Port agentfdr spawn attribution and inline-sidechain nodes with tests (MIT); resolve subagent type from .meta.json, file-name label or the spawning call's subagent_type; labeled internal forks (task_summary, compact) are background forks; guardian trunk reviews are children included by --scope descendants.
