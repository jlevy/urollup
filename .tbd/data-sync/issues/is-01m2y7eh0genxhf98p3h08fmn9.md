---
type: is
id: is-01m2y7eh0genxhf98p3h08fmn9
title: Stabilize the published PR stack against the governing memory review
kind: task
status: in_progress
priority: 1
version: 9
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
child_order_hints:
  - is-01m2ygfa3ggcph5evsdcgx0w98
  - is-01m2ygfaenzm3zrfran0wbkws6
  - is-01m2ygfarr8vb7gzcf96pwzqg3
  - is-01m2ygfb2tpbr0xrg39ypezkxj
  - is-01m2ygfbcmzqfh932496b7ydch
  - is-01m2yh1gk62q41mxph3mnsx2s8
hold: null
hold_until: null
created_at: 2026-09-20T01:38:47.945Z
updated_at: 2026-09-20T05:18:16.028Z
started_at: 2026-09-20T04:16:32.594Z
---
Governing review: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md. Address R1–R5, complete independent review passes and all discovered correctness fixes, propagate changes through owning PR layers, refresh specs and validation evidence, and obtain complete CI. Track performance acceptance, maintainer decisions, Cursor integration and release follow-ups explicitly; do not close unmet gates.

## Notes

Code stability checkpoint reached. All 11 implementation findings and four Cursor planning findings are fixed and closed; three independent technical review passes and their umbrella are closed. PR4 and PR8 each passed 13 hosted checks; PR10, PR11 and PR12 code revisions each passed 15. Full integrated make check passed all 30 negative gate probes. The final PR12 evidence-only review update is receiving its own CI. Governing review and PR descriptions record exact revisions and measured synthetic evidence. Remaining gates are intentionally open: uro-zrr0 for representative 512 MiB/10 s acceptance, uro-erqo for fresh accepted-head whole-history measurements, consented G1/full-history QA, six maintainer decisions and unverified Claude shapes, uro-knnz for Cursor integration after the implementation stack, then release packaging. No new private-log access or maintainer decision acceptance was assumed; no PRs were merged.
