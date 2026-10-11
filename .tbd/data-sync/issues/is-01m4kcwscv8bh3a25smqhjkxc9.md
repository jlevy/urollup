---
type: is
id: is-01m4kcwscv8bh3a25smqhjkxc9
title: Migrated legacy Codex user forks count their parent's copied prefix again
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
created_at: 2026-10-10T17:13:43.066Z
updated_at: 2026-10-11T06:44:03.575Z
started_at: 2026-10-11T06:44:03.044Z
---
Found in PR #32 review B (B3), pre-existing on main. Codex's legacy-to-paginated rollout migration (codex migrate-rollouts, or the background migration feature, since 0.155) drops every session_meta after the head, so a migrated pre-0.152 counter-only user fork (forked_from_id or parent set, no subagent_history_start_ordinal, no foreign header left) keeps its parent's copied counters with nothing marking them as copies. urollup counts them as the fork's own usage: a synthetic case reports 340 instead of 180 with coverage complete. The unmigrated legacy shape is handled by turn-ID copied-history inference. Fix direction: apply the same turn-ID inference against a present parent root that PR #32 round 2 adds for migrated subagents (lines are the parent's until the first turn the parent never recorded, verified by the first-step check), and report a gap when it cannot decide. Needs a synthetic migrated-fork fixture and a real-history check.

## Notes

In progress on branch fix/codex-migrated-forks (worktree .claude/worktrees/codex-migrated-forks): extending PR #32's turn-ID inference to migrated paginated forks without a boundary, history_base or foreign header; tests first in paginated_forks.rs plus a codex-rollout/migrated-user-fork fixture.
