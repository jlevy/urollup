---
type: is
id: is-01m2xgr9vmqwwf9teqq1v0bqnk
title: Shrink Measures in the resident observation shell
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
created_at: 2026-09-19T19:02:11.059Z
updated_at: 2026-09-19T19:08:04.800Z
started_at: 2026-09-19T19:03:27.805Z
closed_at: 2026-09-19T19:08:04.454Z
close_reason: Measures 72 to 36 with interned u64 overflow. Observation 264 to 232, Request 248 to 216. Codex-only 645 MiB / 14.0 s, WH 770 MiB / 18.1 s. Peak fell; still above 512.
resolution: null
duplicate_of: null
---
Grouping is the resident-set peak: every RequestObservation still holds Option<Measures> at 72 B (612k x 72 ~ 42 MiB inside the 264 B shells). Shrink Measures in place without per-row malloc so the fully-resident observation table gets smaller before any Request is built. Do not box, do not u32-truncate without a checked overflow path, do not re-attempt chunked consume (uro-08oj). Peak must fall.

Landed 2026-09-19: counters that fit in u32 stay inline (Measures 72 to 36, Option the same). Counters above u32::MAX intern once per distinct pattern. RequestObservation 264 to 232, Request 248 to 216. Tests green, row counts unchanged. Quiet remasure: Codex-only 660064 KiB (645 MiB) / 14.0 s (one loaded 711760 KiB outlier); whole history 788704 KiB (770 MiB) / 18.1 s. Standing uro-3y9m remasure was Codex 659 MiB / 13.7 s and WH 786 MiB / 18.8 s. Peak fell. Still above 512 MiB.
