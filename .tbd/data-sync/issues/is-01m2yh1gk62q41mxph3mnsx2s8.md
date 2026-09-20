---
type: is
id: is-01m2yh1gk62q41mxph3mnsx2s8
title: Align selected report coverage with counted-request completeness
kind: bug
status: in_progress
priority: 2
version: 4
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:26:27.301Z
updated_at: 2026-09-20T04:32:32.129Z
---
R11, PR4: query CoverageSummary.requests_without_usage documents counted requests but coverage_summary copies global reconcile counter including CopyOnly and unselected threads. Derive missing-usage coverage from selected counted requests and align completeness with SelectionTotals. Preserve copy exclusion and diagnostics; do not invent counted usage or silently settle broader missing-original policy. Add public report tests for counted missing, copy-only, and excluded unrelated missing requests. Review golden changes.

## Notes

Implemented in detached PR4 worktree. Report missing-usage count now derives from selected counted requests, and completeness propagates existing ledger_totals/selection_totals semantics (no new copy policy). Regression red/green covers unmatched copies, counted missing usage, healthy selections excluding missing usage/gaps, and selected unresolved usage. Full workspace Rust tests and all-target clippy passed with isolated debug0 target. Across 29 fixture reports, exactly progress-nested-subagent and legacy-subagent-prefix change missing usage 1 to 0 and complete false to true under CURRENT accounting semantics; root to regenerate/review goldens. Maintainer missing-original policy remains separate: if copy-only should mark incomplete, add a dedicated accounting completeness reason while keeping counted missing-usage count zero.
