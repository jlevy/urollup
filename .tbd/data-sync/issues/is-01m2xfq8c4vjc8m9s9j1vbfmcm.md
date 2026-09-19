---
type: is
id: is-01m2xfq8c4vjc8m9s9j1vbfmcm
title: Shrink Codex per-rollout observation capacity after worker observe
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
created_at: 2026-09-19T18:44:08.195Z
updated_at: 2026-09-19T20:56:09.969Z
started_at: 2026-09-19T18:44:12.990Z
closed_at: 2026-09-19T20:56:09.969Z
close_reason: "Peak did not fall or the design was abandoned. Field and ID relocation are exhausted (uro-t8ws standing 653/586). Do not retry two-pass Claude, prefix-merge, boxing shells, dropping KeyGraph, one-inline-key, per-rollout shrink_to_fit, or chunked consume. Tail-consume after grouping was not filed: the grouping peak holds every shell before Requests are reserved, and shrink_to_fit of that remainder reallocs while the table is still live (already raised RSS)."
resolution: canceled
duplicate_of: null
---
Usage-bearing Codex rollouts reserve RequestObservation slots for every usage, compacted, and token-count line (observation_slots), then often push fewer rows. Those oversized Vecs sit in the try_read_in_parallel join result until normalize appends them. On the worker, after observe, shrink_to_fit the observation, limit, and diagnostic vecs so join RSS is actual rows, not slot slack. Do not box observations, do not shrink_to a large remaining reconcile table, do not prefix-merge. Peak must fall.

Tried 2026-09-19: shrink_to_fit after observe. Tests green, row counts unchanged. Under load about 14: Codex-only 748512 KiB (731 MiB) / 18.5 s; whole history 848112 KiB (828 MiB) / 24.9 s. Baseline after uro-3y9m was Codex-only 650 MiB and WH 647-770 MiB. Peak rose; shrink_to_fit realloc of each rollout table added allocator slack. Reverted.
