---
type: is
id: is-01m2y5dtedbdwka9v6mwcdkass
title: Reconcile memory-budget docs and remeasure current accepted heads
kind: task
status: in_progress
priority: 2
version: 8
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
parent_id: is-01m2pkgva22qd452me3cdjc1fq
child_order_hints:
  - is-01m3jw5tx0qmnp6pqp70hnjpmp
hold: null
hold_until: null
created_at: 2026-09-20T01:03:27.691Z
updated_at: 2026-09-28T02:05:51.892Z
started_at: 2026-09-20T04:28:39.754Z
---
Documentation consistency distinguishes row admission from process RSS, actual CI scale execution, and process-global intern-table lifetime. Under the approved 2026-09-27 memory policy, retain local accepted-head evidence for sessions, daily and report, distinguishing sampled RSS, maximum RSS and physical footprint, host contention/cache state and synthetic versus real corpora. Exploratory live runs completed in the review session, but source growth means they do not establish immutable-boundary worker parity or full acceptance. Preserve evidence outside disposable scratch; no private values are published. Remaining: exact reproducible input-boundary one/eight-worker equivalence, complete representative evidence, and validation of the revised process-wide budget after uro-6pi8 lands. The old 512 MiB/10-second gates are retired. Historical measurements are not current-head evidence.

## Notes

2026-09-20 readiness refresh: row-budget wording, measured-memory distinctions, CI wiring and intern-table lifetime documentation are corrected in the published stack. New Ubuntu/macOS scale gates pass; the earlier missing-wiring statement describes the original audit. Remaining: consented exact-head sessions/daily/report measurements, one/eight-worker equivalence, and representative evidence. Remains open; historical 653/586 MiB numbers are not updated-head measurements.
