---
type: is
id: is-01m4h7ffvb4wz9tcw4fmbj2y87
title: Report request-level token availability in the local aggregate
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies: []
created_at: 2026-10-09T21:00:35.561Z
updated_at: 2026-10-09T21:00:35.561Z
---
Follow-up to PR #24 review A2 (https://github.com/jlevy/urollup/pull/24#pullrequestreview-5474469420). urollup.local-aggregate/v2 labels token fields by day rows (totals.token_day_coverage: all_days/some_days/no_days). A daily row carries a field when any of its counted requests reported it, so on days mixing Claude and Codex requests agent-specific fields such as reasoning read all_days although some requests lacked them. Once query rows carry per-metric reporting request counts (the metric coverage contract in uro-qvp1, with cache-request coverage in uro-381f), mark a metric fully observed only when its count equals the row's request count, and version the aggregate record.
