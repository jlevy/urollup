---
type: is
id: is-01m2xcfaq8tx365vxpxfqshk18
title: Reserve Claude observations per chunk after records drop
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
created_at: 2026-09-19T17:47:22.717Z
updated_at: 2026-09-19T17:55:22.771Z
started_at: 2026-09-19T17:47:26.666Z
closed_at: 2026-09-19T17:55:22.769Z
close_reason: "Landed: claude_project.rs reconcile_input reserves the observation Vec per RecordChunk after earlier ParsedRecord vectors drop, instead of with_capacity(sum of all records) beside the full corpus and the resident Codex ledger. Fixture adapters/snapshots/worker-count, cargo test --workspace (default and --no-default-features), goldens, e2e-results, parity and scale-gate passed. Privacy-safe sessions --all: 796 MiB (815008 KiB) / 81.3 s at load 117–193, same row counts (Codex 612561/611314, Claude 413742/191476). That is below the loaded Box (840 MiB) and RecordRefs (827–852 MiB) A/B; wall time is load, not the cut. Uncommitted per user rule. Still above 512 MiB."
resolution: null
duplicate_of: null
---
After uro-cbsg, whole-history peak is still well above 512 MiB (quiet-machine 804 MiB; loaded A/B ~830–850 MiB). Claude ingest still holds every usage-bearing ParsedRecord in Corpus.chunks, then reconcile_input does Owners / ambiguous / thread_graph and Vec::with_capacity(sum of chunk.records.len()) before any chunk is dropped. RequestObservation is 264 bytes; 413,742 Claude observations is about 104 MiB allocated while the record table and the resident Codex ledger (611,314 Request rows) are all alive.

Reserve the observation vector per RecordChunk after earlier chunks have been consumed, so the 264-byte slots are not allocated beside the full corpus. Do not re-decode (uro-l3fw already raised peak).

Files: claude_project.rs reconcile_input.

Acceptance: fixture snapshots and worker-count identity unchanged; make test; privacy-safe sessions --all on uro-l0gd. Peak must fall.
