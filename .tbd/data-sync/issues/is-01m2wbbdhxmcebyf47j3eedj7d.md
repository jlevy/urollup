---
type: is
id: is-01m2wbbdhxmcebyf47j3eedj7d
title: Observe Codex without retaining every decoded source
kind: task
status: in_progress
priority: 1
version: 3
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
updated_at: 2026-09-19T08:15:35.300Z
started_at: 2026-09-19T08:15:35.299Z
---
Codex ingest collects every ParsedSource (records plus interned strings) before normalize. Observe then frees records per source, but the peak includes the whole decoded corpus plus growing observations. known_turns is a two-pass over root rollouts, so workers cannot observe in isolation without a merge of turn IDs first.

Extract the small known_turns and session-meta tables first, then observe while draining sources so full record vectors do not all coexist with the observation vec. Prefer doing the observe conversion on the worker after a cheap first-pass merge if that stays worker-count identical.

Acceptance: worker-count identity holds; whole-history Codex ingest peak drops; make check.
