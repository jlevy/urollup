---
type: is
id: is-01m5nfzthfk9yxcbxwfej0phrm
title: Narrow --current discovery to the selected session
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - performance
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:58.958Z
updated_at: 2026-10-23T23:01:58.958Z
---
report --current spends 2-5 seconds in discovery on the maintainer's corpus (about 19,000 log files) before narrowing to one session family, against the 1-second small-report target. Locate the current session's files from the agent's own layout (Claude Code project directory and session ID, Codex rollout name) before walking every root, falling back to the full walk only when needed; record timing in UROLLUP_STATS.
