---
type: is
id: is-01m4kcwscv8bh3a25smqhjkxc9
title: Migrated legacy Codex user forks count their parent's copied prefix again
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-10T17:13:43.066Z
updated_at: 2026-10-10T17:13:43.066Z
---
Found in PR #32 review B (B3), pre-existing on main. Codex's legacy-to-paginated rollout migration (codex migrate-rollouts, or the background migration feature, since 0.155) drops every session_meta after the head, so a migrated pre-0.152 counter-only user fork (forked_from_id or parent set, no subagent_history_start_ordinal, no foreign header left) keeps its parent's copied counters with nothing marking them as copies. urollup counts them as the fork's own usage: a synthetic case reports 340 instead of 180 with coverage complete. The unmigrated legacy shape is handled by turn-ID copied-history inference. Fix direction: apply the same turn-ID inference against a present parent root that PR #32 round 2 adds for migrated subagents (lines are the parent's until the first turn the parent never recorded, verified by the first-step check), and report a gap when it cannot decide. Needs a synthetic migrated-fork fixture and a real-history check.
