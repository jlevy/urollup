---
type: is
id: is-01m2kt2hgv0615fba2cf0hfgy0
title: Add --sessions-from, --latest, --agent, --project, --cwd and --whole-sessions
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:32:39.447Z
updated_at: 2026-09-16T03:02:01.740Z
closed_at: 2026-09-16T03:02:01.738Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-feyo is the original.
resolution: duplicate
duplicate_of: is-01m2ksxdf642wffc1dmd7ta9n7
---
Milestone 0.5: the remaining selection flags. Design §6.1 and §6.2; --sessions-from and --whole-sessions are Candidate (§9.1 CLI Surface).

Acceptance:
- --sessions-from <file> reads session selectors one per line; --agent, --project and --cwd filter; repeated values of one flag union and different flags intersect.
- --whole-sessions reports all usage of every session with usage inside the interval; otherwise time filters clip a session's usage to the interval, and on summary input they clip to 15-minute bucket boundaries with a precision diagnostic.
- --latest is the only heuristic and never runs implicitly, including in interactive terminals: it picks the session whose latest record's cwd equals the working directory, explains the choice on stderr, and exits 2 when another session for that directory was active within 10 minutes.
- --scope still defaults to descendants for session selections and self for filter selections.
- CLI goldens cover concurrent sessions in one directory, worktrees, forks and archived rollouts.
