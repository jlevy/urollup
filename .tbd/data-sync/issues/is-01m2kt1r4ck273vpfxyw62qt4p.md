---
type: is
id: is-01m2kt1r4ck273vpfxyw62qt4p
title: Implement the usage summary writer and reader
kind: task
status: closed
priority: 1
version: 7
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2kt1stedbst3rb3wacb3xxe
  - type: blocks
    target: is-01m2kt1vmnk1ck7fd5d742f603
  - type: blocks
    target: is-01m2kt21w66490bxt0vr6kxsra
  - type: blocks
    target: is-01m2kt2sa9a5gpv2n3x2t3zva8
  - type: blocks
    target: is-01m2kt32v8spyp96g40c8495dc
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:32:13.451Z
updated_at: 2026-09-16T03:01:49.408Z
closed_at: 2026-09-16T03:01:49.405Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-ni7m is the original.
resolution: duplicate
duplicate_of: is-01m2ksvq7csxd3xhwb4zvcgnaj
---
Milestone 0.2: read and write urollup:UsageSummary/v1, with extents, the request index and mergeable measures. Design §5.2, §5.3, §5.6 and Decision 17.

Acceptance:
- One extent per owner thread per export, carrying a digest, the request index (on by default, Decision 17), the nonfinal usage-revision map, 15-minute time buckets and per-extent extensions.
- Every measure is stored in a mergeable form: additive token, request, tool-call and duration counters; request-size count, sum, minimum and maximum plus a log-linear histogram (eight buckets per power of two, each at most 12.5% wide); maxima; a top-N largest-requests list; stored busy intervals so concurrent sessions are not double-counted.
- Writers emit explicit nulls for unknown values, quote every timestamp, and keep money as exact decimal strings; percentiles read from a merged histogram report their bucket bounds.
- Reader version rules (§5.6): full support for known major versions at any revision up to its own; a field an older revision lacks is unknown, never zero; a newer revision may be displayed with a diagnostic but never merged or re-exported; unknown major versions exit 2.
- Round-trip and golden tests against the design's current-session and aggregate summary examples (§5.2).
