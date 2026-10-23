---
type: is
id: is-01m5nfzsfxfp7qtzn1tv2e67wf
title: Map worktree working directories to their repository's project
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - cli
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:57.884Z
updated_at: 2026-10-23T23:01:57.884Z
---
Project is the basename of the recorded cwd (claude_project.rs:946), so every git worktree becomes its own project (on the maintainer's corpus, dozens of agent-* worktree names) and same-named repositories merge. Design §2.1 orders the sources: a manifest project mapping, else the git top-level basename when the dialect records it, else the cwd basename. Implement the recorded-field rules available now: Codex session_meta git metadata where present, and Claude Code's own worktree layout (<repo>/.claude/worktrees/<name>) mapped to <repo>; keep the original cwd. Decide whether to show a disambiguating parent for colliding basenames.
