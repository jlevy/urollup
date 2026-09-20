---
type: is
id: is-01m2yh1gk62q41mxph3mnsx2s8
title: Align selected report coverage with counted-request completeness
kind: bug
status: in_progress
priority: 2
version: 3
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:26:27.301Z
updated_at: 2026-09-20T04:28:40.215Z
---
R11, PR4: query CoverageSummary.requests_without_usage documents counted requests but coverage_summary copies global reconcile counter including CopyOnly and unselected threads. Derive missing-usage coverage from selected counted requests and align completeness with SelectionTotals. Preserve copy exclusion and diagnostics; do not invent counted usage or silently settle broader missing-original policy. Add public report tests for counted missing, copy-only, and excluded unrelated missing requests. Review golden changes.
