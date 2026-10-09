---
type: is
id: is-01m4fsy6qtqhqnxfxdqkxzezfw
title: "PR #14 A6: qualify Cursor plan phase names against product phases"
kind: bug
status: closed
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsxxjgrahm03vd1fr1vhms
hold: null
hold_until: null
created_at: 2026-10-09T07:44:43.257Z
updated_at: 2026-10-09T07:50:31.742Z
started_at: 2026-10-09T07:45:10.260Z
closed_at: 2026-10-09T07:50:31.741Z
close_reason: "Fixed in 712016e: the plan qualifies its own Phase 0 and Phase 1 (status line, overview, Locked Format Facts, Dialect row, Implementation Plan intro) as stages of this plan, separate from product Phases 1 to 3; no phases added. Confirmed by pinned flowmark --auto --check . (exit 0), git diff --check (clean), and the relative link and anchor check of the four changed files (907 links, bad=0)."
resolution: null
duplicate_of: null
---
Severity: Low. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: plan-2026-09-19-cursor-dialect.md :6 (status), :158, :204, :359-361, :377.

Problem: the plan's "Phase 0" and "Phase 1" collide with the product plan's phases (product Phase 1 is milestones 0.1-0.5). "Phase 1 picks the token" reads as product Phase 1.

Fix: qualify them as this plan's phases outside the Implementation Plan headings; add no phases.
