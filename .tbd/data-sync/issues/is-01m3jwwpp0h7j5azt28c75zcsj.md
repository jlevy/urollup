---
type: is
id: is-01m3jwwpp0h7j5azt28c75zcsj
title: Preserve source-agent attribution for unowned session rows
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies: []
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-28T02:18:21.245Z
updated_at: 2026-09-28T02:18:21.245Z
---
Code review during real-history QA found query::aggregate::sessions groups all Ownership::Ambiguous and Ownership::Unknown requests under one None thread key, then derives agent only from the missing SessionIndex entry and labels the row unknown. This discards known source-agent provenance and can conflate unowned requests from different agents. Keep ownership uncertainty distinct from agent uncertainty; preserve the known dialect when possible and emit separate unowned groups per agent without guessing session or project ownership. Add a synthetic mixed-agent fixture with ambiguous owners and prove per-agent totals reconcile to the combined total. Coordinate with uro-qvp1 joint grouping. No private source content or aggregate values belong in this bead.
