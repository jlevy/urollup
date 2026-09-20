---
type: is
id: is-01m2yrgcxaf3ewkzv9cjawjf18
title: Produce native tool-call and usage-duration evidence for reports
kind: feature
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m2ksxgyywga6vgp2nmtppfar
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-20T06:36:55.081Z
updated_at: 2026-09-20T06:37:54.270Z
---
Implement the Tools and usage time contract for native Claude/Codex sources. Produce real ToolAction and timing evidence, dedupe replay/resume/forks, preserve names/categories and unknown outcomes, and distinguish observed API duration sums, session span, busy interval unions and tool call-to-result intervals. Do not derive API duration from first/last request observations or label permission/scheduling waits as tool execution time. Clip intervals to calendar windows; recompute non-additive union/span metrics. Feature coverage must expose absent timing/call evidence. Coordinate shared taxonomy/captured-stream code with uro-i6o2 and reporting with uro-rwkw; use synthetic overlap/missing-endpoint fixtures.
