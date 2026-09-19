---
type: is
id: is-01m2xe6nrna10q522hmn4yetgb
title: Drop Codex RequestObservation shells as reconcile builds each request
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
created_at: 2026-09-19T18:17:36.274Z
updated_at: 2026-09-19T20:56:09.956Z
started_at: 2026-09-19T18:18:23.909Z
closed_at: 2026-09-19T20:56:09.956Z
close_reason: "Peak did not fall or the design was abandoned. Field and ID relocation are exhausted (uro-t8ws standing 653/586). Do not retry two-pass Claude, prefix-merge, boxing shells, dropping KeyGraph, one-inline-key, per-rollout shrink_to_fit, or chunked consume. Tail-consume after grouping was not filed: the grouping peak holds every shell before Requests are reserved, and shrink_to_fit of that remainder reallocs while the table is still live (already raised RSS)."
resolution: canceled
duplicate_of: null
---
After uro-3y9m, Codex-only is 650 MiB and whole history 647-770 MiB. reconcile already release_payload keys per group and drop(observations) after the loop, but all 612,561 RequestObservation shells (264 B, about 154 MiB) stay allocated beside the growing Request table (611,314 x 248 B, about 145 MiB). Take or Option-wrap each group out of the observations vec as its Request is built. Files: reconcile.rs request-building loop. Peak must fall.

## Notes

2026-09-19: Boxing RequestObservation at emit and taking boxes during the request loop raised Codex-only sessions --all from 650 MiB (665904 KiB) to 699 MiB (715904 KiB) / 19.3 s. Whole history 649 MiB (664128 KiB) / 84 s under load 83-133. Same row counts (Codex 9851/612561/611314/137670, Claude 1775/413742/191476/384). Reverted. Peak must fall without per-observation Box. Still open.
