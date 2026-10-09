---
type: is
id: is-01m3jwwpp0h7j5azt28c75zcsj
title: Preserve source-agent attribution for unowned session rows
kind: bug
status: in_progress
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-28T02:18:21.245Z
updated_at: 2026-10-09T19:15:14.518Z
---
Code review during real-history QA found query::aggregate::sessions groups all Ownership::Ambiguous and Ownership::Unknown requests under one None thread key, then derives agent only from the missing SessionIndex entry and labels the row unknown. This discards known source-agent provenance and can conflate unowned requests from different agents. Keep ownership uncertainty distinct from agent uncertainty; preserve the known dialect when possible and emit separate unowned groups per agent without guessing session or project ownership. Add a synthetic mixed-agent fixture with ambiguous owners and prove per-agent totals reconcile to the combined total. Coordinate with uro-qvp1 joint grouping. No private source content or aggregate values belong in this bead.

## Notes

In review: PR #25 (https://github.com/jlevy/urollup/pull/25), branch fix/cli-honesty, commit 1991a1d. sessions emits one unowned row per source agent; synthetic ambiguous-owner fixture cases for both dialects plus a mixed-agent golden. The PR also records a pre-existing gap for its own bead: exact --session selection reads only the selected session's files, so each single-session run counts a response that the whole-history run reports as ambiguous (parity ledger entry claude-ambiguous-owner-session-selection). Close when PR #25 merges.
