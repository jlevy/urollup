---
type: is
id: is-01m4fdqwy15hznwazg6cjweps6
title: "Recent Codex usage records lose their model: context is looked up by root_turn_id"
kind: bug
status: in_progress
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T04:11:33.695Z
updated_at: 2026-10-09T05:35:59.510Z
---
usage_observation applied turn context with payload.root_turn_id, but turns are keyed by turn_context.turn_id. In multi-agent subagent rollouts every token_usage_record carries turn_id and root_turn_id; turn_id matches the subagent's turn_context while root_turn_id names the parent's turn, so model and effort were never applied and every subagent request reported model and effort unknown. Top-level rollouts, where the two IDs agree, were unaffected. Fix: store the record's own turn_id, falling back to root_turn_id, in the same slot. Blocks list-price estimates (uro-neii), which need the model.

## Notes

Implemented in PR https://github.com/jlevy/urollup/pull/18 (branch fix/compressed-sources-codex-usage, based on main; also applies cleanly on PR 16 with its fork tests passing). Local gates pass except gate proofs and MSRV tests, left to CI. Close when PR 18 merges.
