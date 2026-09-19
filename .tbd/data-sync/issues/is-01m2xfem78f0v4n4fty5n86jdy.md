---
type: is
id: is-01m2xfem78f0v4n4fty5n86jdy
title: Drop KeyGraph after request grouping
kind: task
status: in_progress
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
hold: null
hold_until: null
created_at: 2026-09-19T18:39:25.415Z
updated_at: 2026-09-19T18:53:11.257Z
started_at: 2026-09-19T18:39:29.910Z
---
After grouping, KeyGraph (HashMap of about 612k AnalyticalIds plus ids/checks/parent vecs, tens of MiB) stays alive beside the observation table and the reserved Request vec. Non-split build_request does not use the graph. Drop it after order is computed; conflicting-key splits build a fresh local KeyGraph. Files: reconcile.rs after the order sort, build_request. Do not box observations. Peak must fall.

Tried 2026-09-19: implemented drop-after-group. Tests green, row counts unchanged (Codex 9851 / 612561 / 611314 / 137670). Under load about 12-14: Codex-only 685712 KiB (669 MiB) / 18.1 s; whole history 838192 KiB (818 MiB) / 20.2 s. Remasured uro-3y9m baseline: Codex-only 659 MiB / 13.7 s, WH 786 MiB / 18.8 s. Peak did not fall; reverted to isolate the next cut. Peak is still Codex ingest plus the observation and request tables, not the KeyGraph overlap.
