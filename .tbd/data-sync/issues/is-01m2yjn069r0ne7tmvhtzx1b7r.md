---
type: is
id: is-01m2yjn069r0ne7tmvhtzx1b7r
title: Qualify unreconciled Cursor JSONL coverage aggregates
kind: bug
status: in_progress
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels: []
dependencies: []
parent_id: is-01m2y50zntpg0bavafh8547sfb
created_at: 2026-09-20T04:54:34.440Z
updated_at: 2026-09-20T04:57:35.228Z
---
PR14 states 76 of 1022 composers have parent JSONL but 935 have no JSONL. Qualify unexplained aggregate discrepancy without inventing an overlap or scanning private logs.

## Notes

Implemented in cursor-dialect c8befd9. Research and plan retain existing survey aggregates but explicitly mark exact missing-session count unverified; no new private scan or invented overlap. Pinned Flowmark auto/check passes. Awaiting parent integration/review.
