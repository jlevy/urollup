---
type: is
id: is-01m4mtk6em9hcwff4chpvydxjy
title: Claude subagent transcripts copied into a resumed session make their requests ambiguous
kind: bug
status: in_progress
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
delegate: claude-code@spud10
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-10-11T06:32:23.251Z
updated_at: 2026-10-11T06:34:38.553Z
started_at: 2026-10-11T06:34:34.632Z
---
Found 2026-10-10 by a local real-history check on main 0f0f6df (qualitative; values kept local). Claude Code can leave a byte-identical copy of a subagent transcript (<session>/subagents/agent-<id>.jsonl) under another session's subagents/ directory, apparently when a session is resumed or forked; every line of the copy still carries the original session's sessionId. urollup derives the subagent thread from the file's location, so both copies prove ownership of the same responses: every such request becomes ambiguous (unowned row), conflicting-owners diagnostics are raised, and whole-history Claude coverage turns partial. In the maintainer's history a handful of copied subagent files cause all ambiguous Claude requests. Fix direction: identify a Claude subagent thread by the lines' sessionId plus agentId (or treat a subagent file whose lines name another session as a copy of that session's subagent), so identical copies merge as one thread and own their requests once; keep genuine conflicts ambiguous. Needs a synthetic fixture (a session, its subagent, and a resumed session holding a copy), goldens, design §3.4 text and a real-history rerun.

## Notes

In progress (2026-10-10): branch fix/claude-copied-subagents in worktree .claude/worktrees/claude-copies off main 0f0f6df. Plan: identify a Claude subagent thread by its lines' sessionId plus agentId so byte-identical copies under a resumed session merge as one thread; synthetic fixture, goldens, design 3.4 and release record entry.
