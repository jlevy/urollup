---
type: is
id: is-01m2xg2vek9txpj40mvab5rp12
title: Shrink RequestObservation keys to one inline slot
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
created_at: 2026-09-19T18:50:28.178Z
updated_at: 2026-09-19T18:53:10.911Z
started_at: 2026-09-19T18:50:31.440Z
---
Adapters push one request key per observation. The second inline DerivedKey slot is 27 bytes on every shell (about 16 MiB on 612k Codex rows) and sits beside the Request table through reconcile. Change RequestObservation.keys to InlineList<DerivedKey, 1>; a second key spills. Do not box observations. Peak must fall.

Tried 2026-09-19: keys N=1, size assert 240. Tests green, row counts unchanged. Codex-only 742672 KiB (725 MiB) / 14.1 s; whole history 818960 KiB (800 MiB) / 18.2 s. Remasured uro-3y9m baseline on the same machine was Codex-only 674416 KiB (659 MiB) / 13.7 s and WH 804512 KiB (786 MiB) / 18.8 s. Peak rose. Claude observe often pushes a message key and a request_id key, so N=1 spills. Reverted.
