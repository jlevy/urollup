---
type: is
id: is-01m2yrgbar7pmmncksknephs5s
title: Implement joint calendar, agent, provider and model grouping
kind: feature
status: open
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrhe5crqpp22w45b3243ev
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m2y1r66p1rgzn4qqcyt61krw
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-20T06:36:53.463Z
updated_at: 2026-09-20T06:46:50.871Z
---
Implement the Joint grouping and metric coverage contract. One grouping tuple supports local day/week/month crossed with agent/provider/model; explicit timezone and week-start, null/ambiguous groups and metric coverage survive. Label existing independent breakdowns separately. Provider has observed/mapped/unknown basis and versioned mappings; agent is not provider and billing channel is separate. Test DST, non-hour offsets, week/month boundaries, absent metadata and joint-to-marginal reconciliation. Own generic facets now; later Cursor bead uro-2qxq should reuse them without pulling database ingestion earlier.

## Notes

Joint token categories reconcile additively, but distinct logical-request counts can overlap across model/tool groups when one request has multiple model usage components. Preserve advisor components and distinguish per-component usage from distinct requests; label non-additive counts and use the request index for exact regrouping. Cache-hit numerator/denominator populations must have the same declared scope.
