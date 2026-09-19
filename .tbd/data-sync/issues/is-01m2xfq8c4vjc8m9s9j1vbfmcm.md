---
type: is
id: is-01m2xfq8c4vjc8m9s9j1vbfmcm
title: Shrink Codex per-rollout observation capacity after worker observe
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
created_at: 2026-09-19T18:44:08.195Z
updated_at: 2026-09-19T18:47:08.724Z
started_at: 2026-09-19T18:44:12.990Z
---
Usage-bearing Codex rollouts reserve RequestObservation slots for every usage, compacted, and token-count line (observation_slots), then often push fewer rows. Those oversized Vecs sit in the try_read_in_parallel join result until normalize appends them. On the worker, after observe, shrink_to_fit the observation, limit, and diagnostic vecs so join RSS is actual rows, not slot slack. Do not box observations, do not shrink_to a large remaining reconcile table, do not prefix-merge. Peak must fall.

Tried 2026-09-19: shrink_to_fit after observe. Tests green, row counts unchanged. Under load about 14: Codex-only 748512 KiB (731 MiB) / 18.5 s; whole history 848112 KiB (828 MiB) / 24.9 s. Baseline after uro-3y9m was Codex-only 650 MiB and WH 647-770 MiB. Peak rose; shrink_to_fit realloc of each rollout table added allocator slack. Reverted.
