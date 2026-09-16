---
type: is
id: is-01m2ksvq7csxd3xhwb4zvcgnaj
title: Implement the usage summary writer and reader
kind: task
status: open
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2ksvsrxm9pqq4yhes5gfxce
  - type: blocks
    target: is-01m2ksvvwqgtym64mz080m0n6m
  - type: blocks
    target: is-01m2ksw5c3g28x3wk3g9hhfybr
  - type: blocks
    target: is-01m2ksy1d1k2de021ff48cwmgb
  - type: blocks
    target: is-01m2ksyv7sj854y8xb6gh0sqyg
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:28:55.897Z
updated_at: 2026-09-16T00:30:38.319Z
---
Milestone 0.2: read and write urollup:UsageSummary/v1, with extents, the request index and mergeable measures. Design §5.2, §5.3, §5.6 and Decision 17.

Acceptance:
- One extent per owner thread per export, carrying a digest, the request index (on by default, Decision 17), the nonfinal usage-revision map, 15-minute time buckets and per-extent extensions.
- Every measure is stored in a mergeable form: additive token, request, tool-call and duration counters; request-size count, sum, minimum and maximum plus a log-linear histogram (eight buckets per power of two, each at most 12.5% wide); maxima; a top-N largest-requests list; stored busy intervals so concurrent sessions are not double-counted.
- Writers emit explicit nulls for unknown values, quote every timestamp, and keep money as exact decimal strings; percentiles read from a merged histogram report their bucket bounds.
- Reader version rules (§5.6): full support for known major versions at any revision up to its own; a field an older revision lacks is unknown, never zero; a newer revision may be displayed with a diagnostic but never merged or re-exported; unknown major versions exit 2.
- Round-trip and golden tests against the design's current-session and aggregate summary examples (§5.2).
