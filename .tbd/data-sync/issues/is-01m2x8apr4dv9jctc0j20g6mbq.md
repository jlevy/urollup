---
type: is
id: is-01m2x8apr4dv9jctc0j20g6mbq
title: Profile whole-history decode throughput and measured regressions
kind: task
status: in_progress
priority: 2
version: 9
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - performance
  - follow-up
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
hold: null
hold_until: null
created_at: 2026-09-19T16:34:56.899Z
updated_at: 2026-09-27T07:36:32.463Z
started_at: 2026-09-19T21:07:49.244Z
---
Profile-led throughput follow-up under the 2026-09-27 policy. There is no fixed 10-second whole-history release target. Record raw bytes, observations, command phases, load and cache conditions; improve measured bottlenecks without changing accounting or raising peak footprint. The historical profile found Codex worker decode dominant. Retain regression review against equivalent corpora; do not retry reverted zero-copy acceptance, larger BufReader, primitive-number or mimalloc changes without a new measured hypothesis. Release acceptance remains on uro-zrr0, with density-scale evidence uro-z1h1 and process-wide safety uro-6pi8.

## Notes

2026-09-19: Unblocked from uro-n1cp. Standing wall 17.3 s.

2026-09-19 profile (privacy-safe). Release quiet remasure: WH 648 MiB / 18.2 s; Codex-only 573 MiB / 14.3 s. The ~7 s over 10 s is Codex worker decode (kernel read ~40%, Line::read ~12%, memmem ~10%).

2026-09-19: uro-s5vb reverted (zero-copy accept: WH 650 / 20.7, Codex 583 / 15.3).
2026-09-19: uro-h6iw reverted (1 MiB BufReader: WH 635 / 21.2, Codex 602 / 13.7). Do not retry a larger sequential window. posix_fadvise was not added.
