---
type: is
id: is-01m4hsgeeskftt5efm505njecp
title: Codex forks and subagents drop counter-only usage beside usage records and recount nested legacy prefixes
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-10T02:15:41.266Z
updated_at: 2026-10-10T02:15:41.266Z
---
PR #26 (uro-h2sf) counts Codex counter-only usage beside token_usage_record lines in root rollouts only: rollouts whose every session_meta is their own, with no parent_thread_id, forked_from_id or subagent_history_start_ordinal, and whose settings events and usage records name no other thread. Forks and subagents keep main's accounting, which leaves these gaps (synthetic shapes in crates/urollup-core/tests/codex_mixed_usage.rs; review C on PR #26: https://github.com/jlevy/urollup/pull/26#pullrequestreview-5476922038):

1. Counter-only turns in a fork or subagent that also has usage records are not counted, with complete coverage and no diagnostic: a legacy subagent or fork resumed by Codex 0.153+ (its pre-upgrade own turns), a declared-boundary child resumed after the upgrade, and the resumed subagent in review C's c6. Behind a non-root parent (c1c) the legacy copied prefix never ends, so the child's counter-only turns are copies even without usage records.
2. Nested legacy chains (review C's c2b, present on main): in an all-legacy fork of a legacy fork or subagent, the turn-ID inference ends the grandchild's copied prefix at the intermediate thread's first own turn, which the root never recorded, so the intermediate thread's usage counts again as the grandchild's.

Likely fix (review C, C1 option a): end a legacy copied prefix only at a turn that the rollout's parent thread (parent_thread_id or forked_from_id) never recorded in any of its rollouts, which needs KnownTurns for every thread, not only roots; then apply the root-rollout adjacency rule to forks and subagents, keying a copied twin only within its own thread (review C, C2). Guardian-review ownership is uro-jqc3, not this bead.
