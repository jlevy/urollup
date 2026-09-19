---
type: is
id: is-01m2xe6nrna10q522hmn4yetgb
title: Drop Codex RequestObservation shells as reconcile builds each request
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T18:17:36.274Z
updated_at: 2026-09-19T18:17:43.211Z
---
After uro-3y9m, Codex-only is 650 MiB and whole history 647-770 MiB. reconcile already release_payload keys per group and drop(observations) after the loop, but all 612,561 RequestObservation shells (264 B, about 154 MiB) stay allocated beside the growing Request table (611,314 x 248 B, about 145 MiB). Take or Option-wrap each group out of the observations vec as its Request is built. Files: reconcile.rs request-building loop. Peak must fall.
