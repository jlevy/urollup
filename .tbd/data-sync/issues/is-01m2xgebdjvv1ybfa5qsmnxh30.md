---
type: is
id: is-01m2xgebdjvv1ybfa5qsmnxh30
title: Consume Codex observation chunks before reserving Requests
kind: task
status: in_progress
priority: 1
version: 4
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
updated_at: 2026-09-19T19:02:10.910Z
started_at: 2026-09-19T18:56:47.604Z
---
Codex already observes per rollout, then flatten-appends 612k shells into one Vec before reconcile reserves the Request table beside it. Keep those per-rollout Vecs as ReconcileInput.request_chunks. After grouping, process groups whose largest member-chunk is this rollout (largest chunks first so other members still live), reserve Request capacity only for that chunk's groups, then drop the chunk. Do not box shells, do not shrink_to a large remainder, do not drop KeyGraph after grouping. Peak must fall.

Tried 2026-09-19: per-rollout request_chunks plus consume-by-home-chunk. Tests green, row counts unchanged. First Codex-only 667328 KiB (652 MiB) / 16.6 s looked like a small fall vs 674416 KiB (659 MiB); whole history rose to 980896 KiB (958 MiB) / 26.1 s. After replacing per-group Vecs with ranges into order: Codex-only 693776 KiB (677 MiB) / 14.8 s and WH 949728 KiB (927 MiB) / 22.0 s. Grouping still holds every shell, so the Request overlap was not the peak; 9851 live tables plus loc/home bookkeeping raised whole-history RSS. Reverted.
