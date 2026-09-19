---
type: is
id: is-01m2wbbfnm8gs10hrmyrg14tma
title: Measure whole-history RSS after each 512 MiB compaction
kind: task
status: closed
priority: 2
version: 22
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2pkgts2b87n25929xphbnpc
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T08:08:33.712Z
updated_at: 2026-09-19T20:58:32.662Z
closed_at: 2026-09-19T20:58:32.661Z
close_reason: "Dated privacy-safe remasure is in the scalable-ingestion Progress table: after uro-t8ws, Codex-only 586 MiB (600336 KiB) / 13.2 s and whole history 653 MiB (668384 KiB) / 17.3 s (repeat 661 / 18.6). Phase 1 field cuts are exhausted. This record does not close uro-n1cp (still 141 MiB over 512)."
resolution: null
duplicate_of: null
---
After each Phase 1 compaction, run privacy-safe whole-history sessions --all (and daily/report --all when the peak changes) on the maintainer corpus. Record peak RSS, wall time, UROLLUP_STATS row counts and worker-count identity. Quote no paths, IDs or prompt text. Update the scalable-ingestion spec Progress table and the 512 MiB checkbox when the goal is met.

Acceptance: dated privacy-safe numbers in the spec; uro-n1cp closable only when peak is at or below 512 MiB and make check is green.

## Notes

2026-09-19: Blockers canceled. Standing remasure remains uro-t8ws: Codex-only 586 MiB (600336 KiB) / 13.2 s, WH 653 MiB (668384 KiB) / 17.3 s (repeat 661 / 18.6). No further Phase 1 field cut is queued. Closes uro-n1cp only at ≤512 MiB.
