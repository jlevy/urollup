---
type: is
id: is-01m2wbbcf9be2tbvyz9f7yydnx
title: Compact EvidenceRef to a source index
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T08:08:30.439Z
updated_at: 2026-09-19T08:08:38.326Z
---
EvidenceRef stores a full AnalyticalId (~17–24 B) plus offset and length, about 40 B per reference. The spec budget is 16 B via a source u32 into a side table. Observation and request rows carry these refs at corpus scale (~1.00M observations).

Replace the embedded source ID with a compact index, keep a source table on Ingested or the ledger, and update adapters, diagnostics and tests. Size-budget asserts should move with the type.

Acceptance: RequestObservation and request evidence shrink; fixture snapshots and goldens stay semantically identical; make check.
