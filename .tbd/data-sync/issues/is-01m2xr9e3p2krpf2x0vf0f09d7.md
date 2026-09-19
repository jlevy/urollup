---
type: is
id: is-01m2xr9e3p2krpf2x0vf0f09d7
title: Skip full JSON walk on Codex lines the type prefilter rejects
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
    target: is-01m2x8apr4dv9jctc0j20g6mbq
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-19T21:13:52.502Z
updated_at: 2026-09-19T21:13:58.217Z
---
Phase 2 speed leftover named by the uro-lsaz profile. Codex-only wall is worker decode, not grouping.

On a sampling profile of the profiling binary (Codex-only, 8 workers, no paths or IDs): about 40% of each worker is kernel read, about 27% is SourceDecoder::decode calling validate_record on lines may_be_relevant rejected, about 12% is Line::read of relevant lines, about 10% is memmem. reconcile.rs is negligible. Main thread spends most of the run in pthread_join.

validate_record (sources/decode.rs) walks every skipped content line with serde_json deserialize_any so it matches parse_record. Those lines are already known not to be usage types. Do not drop the malformed-line contract (design §2.2) without an equivalent cheaper check. This is not CompactJson-without-Value (uro-nuhn) and not Claude Value::from number checks (uro-nzo1).

Files:
- crates/urollup-core/src/adapters/codex_rollout.rs SourceDecoder::decode (the validate_record call at the prefilter miss)
- crates/urollup-core/src/sources/decode.rs validate_record / Skip

Acceptance: Codex-only and whole-history wall fall; malformed counts stay correct or the cheaper check is proven equivalent; make check; revert if wall does not fall or peak rises.
