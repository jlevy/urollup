---
type: is
id: is-01m2jzdtyxq3edt2kjr2am8bwh
title: Create unified urollup design doc and refocus plan spec
kind: task
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - planning
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-15T16:46:57.998Z
updated_at: 2026-09-15T17:52:55.641Z
started_at: 2026-09-15T16:46:59.428Z
closed_at: 2026-09-15T17:52:55.640Z
close_reason: "Design doc created in feef940 (docs/urollup-design.md), architecture doc removed, plan refocused; senior review loop on PR #3 completed in 75228c5 and 017fc62."
resolution: null
duplicate_of: null
---
Create docs/urollup-design.md as the single living design specification, modeled on jlevy/tbd packages/tbd/docs/tbd-design.md (numbered layers, table of contents, status labels for confirmed and candidate behavior, design decisions appendix, cross-cutting open questions). Move all design content from the plan spec's Design section and the data contracts architecture doc into it without losing constraints, remove the architecture doc, refocus the plan spec on milestones, testing and rollout with references into the design doc, and fix links across README, AGENTS.md, research briefs and explorations.
