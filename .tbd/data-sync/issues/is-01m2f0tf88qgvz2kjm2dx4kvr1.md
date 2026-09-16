---
type: is
id: is-01m2f0tf88qgvz2kjm2dx4kvr1
title: Implement current-session detection, --session, --all and hierarchy crawler
kind: task
status: open
priority: 2
version: 9
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2f0tg7kbpmm11wycct2vb5n
  - type: blocks
    target: is-01m2kefnq411wpc8xfmxcq8tjm
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-14T03:54:22.855Z
updated_at: 2026-09-16T00:20:59.482Z
---
Milestone 0.1: --current environment detection (CLAUDE_CODE_SESSION_ID, CODEX_THREAD_ID; a detected Pi session exits 2 with an unsupported-dialect diagnostic), --session, --all, the discovery index and hierarchy crawler behind --scope self|descendants, per design §6.1, §6.2 and §3.2. Remaining selection flags are milestone 0.5.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Codex hook input: check the rollout by session_meta.session_id == session_id and, when agent_id is present, session_meta.id == agent_id; on SubagentStop use agent_transcript_path; hooks get no CODEX_* variables; detect nested agents by comparing hook session_id with any CODEX_SESSION_ID in the environment; a null transcript_path is an unsaved session (exit 1).
- Pi: PI_SESSION_ID without PI_SESSION_FILE is unsaved (exit 1); Pi sessions include the calling request.
- For --current, resolve the transcript and its subagents/ directory first and parse only those (ccusage statusline full scans show the cost otherwise).
- Codex SQLite state_*.sqlite threads and thread_spawn_edges as an optional, version-gated discovery hint, never a usage source.
- Port agentfdr spawn attribution and inline-sidechain nodes with tests (MIT); resolve subagent type from .meta.json, file-name label or the spawning call's subagent_type; labeled internal forks (task_summary, compact) are background forks; guardian trunk reviews are children included by --scope descendants.

Must include end-to-end goldens and result checks on the fixture cases (uro-3ht1, uro-xsfj): selection and --scope behavior shows in each case's transcript golden, and the per-case ownership counts are checked against expected.json by `make e2e-results`. Detection goldens set the agent variables in front matter only; the harness scrubs the real ones (tests/golden/README.md).
