---
type: is
id: is-01m2xgebdjvv1ybfa5qsmnxh30
title: Consume Codex observation chunks before reserving Requests
kind: task
status: closed
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
created_at: 2026-09-19T18:56:44.976Z
updated_at: 2026-09-19T20:56:09.984Z
started_at: 2026-09-19T18:56:47.604Z
closed_at: 2026-09-19T20:56:09.984Z
close_reason: "Peak did not fall or the design was abandoned. Field and ID relocation are exhausted (uro-t8ws standing 653/586). Do not retry two-pass Claude, prefix-merge, boxing shells, dropping KeyGraph, one-inline-key, per-rollout shrink_to_fit, or chunked consume. Tail-consume after grouping was not filed: the grouping peak holds every shell before Requests are reserved, and shrink_to_fit of that remainder reallocs while the table is still live (already raised RSS)."
resolution: canceled
duplicate_of: null
---
Codex already observes per rollout, then flatten-appends 612k shells into one Vec before reconcile reserves the Request table beside it. Keep those per-rollout Vecs as ReconcileInput.request_chunks. After grouping, process groups whose largest member-chunk is this rollout (largest chunks first so other members still live), reserve Request capacity only for that chunk's groups, then drop the chunk. Do not box shells, do not shrink_to a large remainder, do not drop KeyGraph after grouping. Peak must fall.

Tried 2026-09-19: per-rollout request_chunks plus consume-by-home-chunk. Tests green, row counts unchanged. First Codex-only 667328 KiB (652 MiB) / 16.6 s looked like a small fall vs 674416 KiB (659 MiB); whole history rose to 980896 KiB (958 MiB) / 26.1 s. After replacing per-group Vecs with ranges into order: Codex-only 693776 KiB (677 MiB) / 14.8 s and WH 949728 KiB (927 MiB) / 22.0 s. Grouping still holds every shell, so the Request overlap was not the peak; 9851 live tables plus loc/home bookkeeping raised whole-history RSS. Reverted.
