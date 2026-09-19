---
type: is
id: is-01m2wbbcf9be2tbvyz9f7yydnx
title: Compact EvidenceRef to a source index
kind: task
status: closed
priority: 1
version: 7
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
  - type: blocks
    target: is-01m2x8ap0h71f5mpry8b57wb3k
  - type: blocks
    target: is-01m2x8apcp21hd5yacp01m0a9b
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T08:08:30.439Z
updated_at: 2026-09-19T17:07:54.033Z
closed_at: 2026-09-19T17:07:54.032Z
close_reason: "EvidenceRef is 16 bytes (u32 source index, u32 length, u64 offset) with custom Ord (source, offset, length). SourceTable on ReconcileInput/Ledger assigns indices in AnalyticalId order; adapters stamp local 0 then remap. artifact_local resolves src- IDs through the table. RequestObservation <= 264 B. make test green. Release sessions --all: 875 MiB (896016 KiB) / 26.5 s, same row counts. Did not meet 512 MiB."
resolution: null
duplicate_of: null
---
EvidenceRef stores a full AnalyticalId (17 B) plus offset and length u64s, about 40 B per reference. The spec budget is 16 B via a source u32 into a side table. Observation and request rows carry these refs at corpus scale (~1.00M observations; Request.records is a Box<[EvidenceRef]>).

Replace the embedded source ID with a compact index, keep a source table on Ingested or the ledger, and update adapters, diagnostics and tests. Size-budget asserts should move with the type.

Files and types:
- crates/urollup-core/src/sources/evidence.rs: EvidenceRef
- crates/urollup-core/src/adapters.rs: Ingested (source table lives here or on Ledger)
- crates/urollup-core/src/ledger/identity.rs: AnalyticalId stays 17 B; it leaves the ref
- crates/urollup-core/src/ledger/reconcile.rs: RequestObservation.evidence (288 B assert)
- crates/urollup-core/src/ledger/entities.rs: Request.records, Thread.evidence, Relationship.evidence, ProviderLimitObservation.evidence, ToolAction.evidence
- crates/urollup-core/src/ledger/diagnostics.rs and coverage.rs
- crates/urollup-core/src/adapters/claude_project.rs and adapters/codex_rollout.rs: ParsedRecord::evidence
- crates/urollup-core/src/sources/reader.rs: Scan::run constructs EvidenceRef
- crates/urollup-core/src/selection.rs: SessionIndex copies then clears thread.evidence
- Tests: crates/urollup-core/tests/snapshots.rs, tests/adapters.rs, ledger/reconcile/tests.rs, sources/reader/tests.rs, ledger/diagnostics.rs tests

Acceptance: RequestObservation and request evidence shrink; fixture snapshots and goldens stay semantically identical; make check.
