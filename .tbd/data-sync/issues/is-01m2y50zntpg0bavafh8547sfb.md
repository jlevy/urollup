---
type: is
id: is-01m2y50zntpg0bavafh8547sfb
title: Reconcile Cursor docs against the accepted implementation stack
kind: task
status: open
priority: 2
version: 7
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
child_order_hints:
  - is-01m2yjmzvq26xhmfth9fezp7my
  - is-01m2yjn069r0ne7tmvhtzx1b7r
  - is-01m2yjn0fjtg72ag044th3pzjp
  - is-01m2yjn0s0jbsvf46c7j128yr2
created_at: 2026-09-20T00:56:27.065Z
updated_at: 2026-09-20T04:57:36.123Z
---
PR #14 remains separate on main and has no Actions checks because main has no workflow. A git merge-tree simulation against ingest-capacity d446732 conflicts in the product plan: retain both the full End-to-End Acceptance Goals section and the later-Cursor pointer. Cursor plan/research link to implementation files and the scalable-ingestion spec absent on current main. After the implementation stack lands, rebase Cursor onto main, resolve the conflict, check local links and privacy/synthetic-only claims, run docs CI, and record review. Do not mix Cursor adapter implementation into 0.1. Local data claims were not independently re-surveyed by this audit. Spec on branch cursor-dialect: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md.

## Notes

Independent docs review fixed four findings in c8befd9 (uro-g7da uro-6unm uro-oxnw uro-1vfu). Local links/heading check has five distinct implementation-dependent targets absent on Cursor base but present on current root: golden README, discovery.rs, scalable-ingestion spec, first-release-publishing spec, and product-plan golden-and-end-to-end-result-checks anchor. No unexplained target failures. Keep rebase and final link/docs CI validation pending implementation merge. No private logs inspected.
