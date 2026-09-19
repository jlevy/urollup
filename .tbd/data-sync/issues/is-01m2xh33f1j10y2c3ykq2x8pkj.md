---
type: is
id: is-01m2xh33f1j10y2c3ykq2x8pkj
title: Pack RequestObservation sequence into 8 bytes
kind: task
status: closed
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
created_at: 2026-09-19T19:08:04.961Z
updated_at: 2026-09-19T19:17:29.941Z
started_at: 2026-09-19T19:08:08.051Z
closed_at: 2026-09-19T19:17:29.940Z
close_reason: "Packed Option<u64> sequence as Option<NativeSequence> (n+1 in NonZeroU64). Observation 232 to 224. Paired A/B vs 7w0u: Codex-only 628608 KiB (614 MiB) / 14.5 s vs 646416 KiB (631 MiB); WH 786656 KiB (768 MiB) / 17.5 s vs 797008 KiB (778 MiB). Row counts unchanged. Tests green. Peak fell. Still above 512."
resolution: null
duplicate_of: null
---
Option<u64> sequence is 16 B on every grouping-resident shell. Store n+1 in Option<NonZeroU64> (8 B) so zero remains representable. Do not box, do not drop keys to N=1. Peak must fall.
