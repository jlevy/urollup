---
type: is
id: is-01m2xqdn0pvctsgvsnhna3w9cn
title: Check Claude usage numbers without Value::from
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
    target: is-01m2x8aq4skat5vmx2y97th32e
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-19T20:58:42.069Z
updated_at: 2026-09-19T20:58:50.236Z
---
Phase 2 leftover on the Claude usage-line path. UnsignedAt and unsigned helpers in crates/urollup-core/src/adapters/claude_project/line.rs wrap each number in Value::from before the same check parse_record would apply. Implement the unsigned/number predicate on i64/u64/f64 directly so UsageBody stays document-free.

Keep exactness with parse_record. Peak or wall must fall; revert if either rises.
