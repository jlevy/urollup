---
type: is
id: is-01m2wbbdhxmcebyf47j3eedj7d
title: Observe Codex without retaining every decoded source
kind: task
status: closed
priority: 1
version: 6
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
created_at: 2026-09-19T08:08:31.525Z
updated_at: 2026-09-19T16:29:13.069Z
started_at: 2026-09-19T08:15:35.299Z
closed_at: 2026-09-19T16:29:13.068Z
close_reason: Usage-bearing Codex rollouts are observed on the decode worker and drop records before join. Observation slots count usage/compacted/token-count only. Whole-history peak 861 MiB (881296 KiB), down ~75 MiB. Remaining peak is Codex decode (rate_limits Value, worker buffers) — uro-zrr0.
resolution: null
duplicate_of: null
---
Codex ingest collects every ParsedSource (records plus interned strings) before normalize. Observe then frees records per source, but the peak includes the whole decoded corpus plus growing observations. known_turns is a two-pass over root rollouts, so workers cannot observe in isolation without a merge of turn IDs first.

Extract the small known_turns and session-meta tables first, then observe while draining sources so full record vectors do not all coexist with the observation vec. Prefer doing the observe conversion on the worker after a cheap first-pass merge if that stays worker-count identical.

Acceptance: worker-count identity holds; whole-history Codex ingest peak drops; make check.

## Notes

2026-09-19: After uro-brar the whole-history peak is Codex ingest (20.8 s, ~936 MiB). RequestObservation is 288 B vs Codex ParsedRecord 80 B, so a drain that still holds remaining records plus growing observations can peak on their sum. Next cut: extract known_turns on the worker, then observe so full record vectors do not coexist with the observation vec (two-wave or per-source observe after a cheap turn-id merge). Worker-count identity must hold. EvidenceRef (uro-as4a) is only tens of MiB and will not reach 512 alone.
