---
type: is
id: is-01m2xqdmmzccchjg7wb9x4kjh5
title: Write Codex CompactJson numbers without Value
kind: task
status: closed
priority: 0
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
  - memory
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-19T20:58:41.694Z
updated_at: 2026-09-19T21:07:06.463Z
started_at: 2026-09-19T20:58:50.564Z
closed_at: 2026-09-19T21:01:57.474Z
close_reason: CompactJson visit_i64/u64/f64 wrote serde_json text from primitives instead of Value::from. Tests stayed green. Quiet WH 685 MiB (701152 KiB) / 21.3 s vs standing 653 / 17.3 (loaded first sample 705 MiB / 25.1 s). Peak and wall both worse. Reverted. Do not retry this scalar-only change.
resolution: canceled
duplicate_of: null
---
Phase 2 leftover on the Codex rate_limits path. CompactJson::visit_i64/u64/f64 in crates/urollup-core/src/adapters/codex_rollout/line.rs still allocates serde_json::Value to print a number. Write the same serde_json text from the primitive (serde_json::to_string on i64/u64/f64) so RateLimitsSeed / DecodedLimits never build a document for a scalar.

Do not change DecodedLimits shape or native JSON contract. Peak or wall must fall; revert if either rises.
