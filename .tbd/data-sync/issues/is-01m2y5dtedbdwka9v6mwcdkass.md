---
type: is
id: is-01m2y5dtedbdwka9v6mwcdkass
title: Reconcile memory-budget docs and remeasure current accepted heads
kind: task
status: in_progress
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2pkgva22qd452me3cdjc1fq
hold: null
hold_until: null
created_at: 2026-09-20T01:03:27.691Z
updated_at: 2026-09-20T05:35:13.089Z
started_at: 2026-09-20T04:28:39.754Z
---
Documentation consistency fixes are implemented: row admission is distinguished from process RSS, CI scale execution is accurately described, and process-global intern-table lifetime is documented. Remaining work is a consented exact-commit table for sessions, daily and report, distinguishing sampled RSS, maximum RSS and physical footprint, loaded versus quiet runs and synthetic versus real corpora. Do not use historical 653/586 MiB measurements as current-head evidence. Track intern-table cardinality/lifetime before repeated library/server use.

## Notes

2026-09-20 readiness refresh: row-budget wording, measured-memory distinctions, CI wiring and intern-table lifetime documentation are corrected in the published stack. New Ubuntu/macOS scale gates pass; the earlier missing-wiring statement describes the original audit. Remaining: consented exact-head sessions/daily/report measurements, one/eight-worker equivalence, and representative evidence. Remains open; historical 653/586 MiB numbers are not updated-head measurements.
