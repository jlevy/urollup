---
type: is
id: is-01m2x8ap0h71f5mpry8b57wb3k
title: Drop Claude payload records after owner-map facts
kind: task
status: in_progress
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies: []
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T16:34:56.143Z
updated_at: 2026-09-19T17:34:02.741Z
---
Contingency if typed Claude decode plus EvidenceRef still miss 512 MiB. Claude cannot observe on the worker: Owners::new and ambiguous_messages need a global pass. Today Corpus keeps every ParsedRecord until reconcile_input walks RecordChunk and frees each chunk only after observe.

Split owner-map facts (message/uuid digests, original-eligible flags, source order) from payload so the global maps can run, then payloads can be observed and dropped without a second full-record table living beside the observation vector.

Files and functions:
- crates/urollup-core/src/adapters/claude_project.rs: ParsedRecord, DecodedSource.records, Corpus, RecordChunk, Owners::new, ambiguous_messages, reconcile_input (the with_capacity over chunk.records.len() and the per-chunk observe loop).
- Do not move Claude observation onto the worker; the owner rule is global.

Acceptance: peak Claude ingest RSS falls without changing fixture totals or worker-count identity; make check; measure on uro-l0gd. Skip if 512 MiB is already met.

## Notes

2026-09-19: Two-pass Claude ingest implemented (OwnerRow/OwnerChunk, decode_facts, try_read_in_parallel_for_each, observe_decoded via OwnerIds). Fixture totals unchanged. Release sessions --all peaked at 929-946 MiB (951200-968496 KiB) / 20.5-26.4 s, same row counts as uro-1sm8. Sequential pass 2 was 908 MiB / 28.0 s. Reverted to single-pass RecordChunk observe. After revert: 804 MiB (823632 KiB) / 22.1 s. try_read_in_parallel_for_each remains in parallel.rs. Acceptance (peak falls) not met. Next cut is uro-cbsg.
