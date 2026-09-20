---
type: is
id: is-01m2x8apr4dv9jctc0j20g6mbq
title: Cut whole-history wall time to 10 seconds
kind: task
status: in_progress
priority: 1
version: 7
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-19T16:34:56.899Z
updated_at: 2026-09-20T00:58:52.378Z
started_at: 2026-09-19T21:07:49.244Z
---
Profile-led Phase 2 work to meet the 10 s whole-history target while uro-zrr0 also owns the unmet 512 MiB target. This work no longer waits for uro-n1cp to close. Standing historical baseline is WH 653 MiB / 17.3 s; later quiet profile 648 MiB / 18.2 s with Codex ingest 13.5 s. Measure first and change only what profiling identifies. Reject changes that regress accounting, peak or wall time. Do not retry reverted zero-copy acceptance, 1 MiB BufReader, primitive-number rewrite or mimalloc cuts. Acceptance: release sessions, daily and report over the representative corpus at most 10 s with the required peak; one and eight workers preserve output; current CI scale gates actually run; record only privacy-safe aggregates.

## Notes

2026-09-19: Unblocked from uro-n1cp. Standing wall 17.3 s.

2026-09-19 profile (privacy-safe). Release quiet remasure: WH 648 MiB / 18.2 s; Codex-only 573 MiB / 14.3 s. The ~7 s over 10 s is Codex worker decode (kernel read ~40%, Line::read ~12%, memmem ~10%).

2026-09-19: uro-s5vb reverted (zero-copy accept: WH 650 / 20.7, Codex 583 / 15.3).
2026-09-19: uro-h6iw reverted (1 MiB BufReader: WH 635 / 21.2, Codex 602 / 13.7). Do not retry a larger sequential window. posix_fadvise was not added.
