---
type: is
id: is-01m4kcwscv8bh3a25smqhjkxc9
title: Migrated legacy Codex user forks count their parent's copied prefix again
kind: bug
status: in_progress
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
delegate: claude-code@spud10
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-10-10T17:13:43.066Z
updated_at: 2026-10-11T07:18:09.875Z
started_at: 2026-10-11T06:44:03.044Z
---
Found in PR #32 review B (B3), pre-existing on main. Codex's legacy-to-paginated rollout migration (codex migrate-rollouts, or the background migration feature, since 0.155) drops every session_meta after the head, so a migrated pre-0.152 counter-only user fork (forked_from_id or parent set, no subagent_history_start_ordinal, no foreign header left) keeps its parent's copied counters with nothing marking them as copies. urollup counts them as the fork's own usage: a synthetic case reports 340 instead of 180 with coverage complete. The unmigrated legacy shape is handled by turn-ID copied-history inference. Fix direction: apply the same turn-ID inference against a present parent root that PR #32 round 2 adds for migrated subagents (lines are the parent's until the first turn the parent never recorded, verified by the first-step check), and report a gap when it cannot decide. Needs a synthetic migrated-fork fixture and a real-history check.

## Notes

In progress: PR #33 (https://github.com/jlevy/urollup/pull/33, branch fix/codex-migrated-forks, not merged). A counter-only paginated rollout with a parent link and no boundary, history_base, subagent thread_source, other thread's header or seeded first counter is judged against an unmarked boundary past its last line with PR #32's turn, total and contradiction rules; undecided steps are a codex-history-boundary-unverified gap. New fixture codex-rollout/migrated-user-fork; existing fixture outputs unchanged. Residual: native counter-only forks (0.150-0.152, history_base) still count their seeded total, as on main.
