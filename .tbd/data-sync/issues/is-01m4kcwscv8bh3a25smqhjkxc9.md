---
type: is
id: is-01m4kcwscv8bh3a25smqhjkxc9
title: Migrated legacy Codex user forks count their parent's copied prefix again
kind: bug
status: in_progress
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
delegate: claude-code@spud10
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-10-10T17:13:43.066Z
updated_at: 2026-10-11T19:17:04.554Z
started_at: 2026-10-11T06:44:03.044Z
---
Found in PR #32 review B (B3), pre-existing on main. Codex's legacy-to-paginated rollout migration (codex migrate-rollouts, or the background migration feature, since 0.155) drops every session_meta after the head, so a migrated pre-0.152 counter-only user fork (forked_from_id or parent set, no subagent_history_start_ordinal, no foreign header left) keeps its parent's copied counters with nothing marking them as copies. urollup counts them as the fork's own usage: a synthetic case reports 340 instead of 180 with coverage complete. The unmigrated legacy shape is handled by turn-ID copied-history inference. Fix direction: apply the same turn-ID inference against a present parent root that PR #32 round 2 adds for migrated subagents (lines are the parent's until the first turn the parent never recorded, verified by the first-step check), and report a gap when it cannot decide. Needs a synthetic migrated-fork fixture and a real-history check.

## Notes

In progress: PR #33 (https://github.com/jlevy/urollup/pull/33, branch fix/codex-migrated-forks, not merged), round 2 at 7449cd4 on main df8c5ac, addressing review A. Unmarked boundary for counter-only paginated forks with a parent link and no boundary, history_base, subagent thread_source or source, other header or seeded first counter. Round 2 fixes a thread-less settings event inside an open inferred start (also PR #32's subagent path) and pins remaining shapes: rollback double counts (uro-eh0d), seeded native forks (uro-p12c), and a fork's own step repeating a parent total (dropped as a copy; no bead yet).
