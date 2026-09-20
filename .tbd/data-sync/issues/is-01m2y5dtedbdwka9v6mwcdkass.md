---
type: is
id: is-01m2y5dtedbdwka9v6mwcdkass
title: Reconcile memory-budget docs and remeasure current accepted heads
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2pkgva22qd452me3cdjc1fq
created_at: 2026-09-20T01:03:27.691Z
updated_at: 2026-09-20T04:24:07.637Z
---
Audit uro-y7zm found stale original-design/as-built estimates, old 2 GiB wording in scale scripts, and claims that the scale workload runs in CI although wiring is absent (uro-a8fk). After audit fixes, update AGENTS/design/spec/QA references together and record one dated exact-commit table for sessions, daily and report; distinguish sampled RSS, maximum RSS and physical footprint, loaded versus quiet samples, synthetic density and real corpus. Do not turn the 25% row budget into a claim of process-RSS enforcement or use historical 653/586 measurements as current-head evidence. Track process-global intern-table cardinality/lifetime before repeated library/server use.
