---
type: is
id: is-01m2xgr9vmqwwf9teqq1v0bqnk
title: Shrink Measures in the resident observation shell
kind: task
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies: []
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T19:02:11.059Z
updated_at: 2026-09-19T19:02:11.059Z
---
Grouping is the resident-set peak: every RequestObservation still holds Option<Measures> at 72 B (612k x 72 ~ 42 MiB inside the 264 B shells). Shrink Measures in place without per-row malloc so the fully-resident observation table gets smaller before any Request is built. Do not box, do not u32-truncate without a checked overflow path, do not re-attempt chunked consume (uro-08oj). Peak must fall.
