---
type: is
id: is-01m2f0th11tcymcgy8zqfqkn2g
title: Establish feature matrix against ccusage and agentfdr
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:24.672Z
updated_at: 2026-09-14T19:58:20.250Z
---
Measured report matrix against pinned ccusage and agentfdr, explaining disagreements from source records rather than treating either tool as an oracle.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Name the exact ccusage command path per row (daily vs monthly, with or without --since, JSON vs table for Codex), pin --offline and --mode calculate, and list expected disagreements: week start, session windows, Codex session identity by path, Pi forks, advisor and progress records, pricing differences.
- Add squares' readers as a labeled third comparator for token and time figures.
