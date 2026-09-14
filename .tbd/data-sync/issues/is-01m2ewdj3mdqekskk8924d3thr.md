---
type: is
id: is-01m2ewdj3mdqekskk8924d3thr
title: "Plan: define the soft-schema session summary format, closed under aggregation"
kind: task
status: closed
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe0xffh11dgc6acjznk3r
  - type: blocks
    target: is-01m2ewe3whtebs3z0pgza28pj4
  - type: blocks
    target: is-01m2ew9s31d10akhg1fafmb83b
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:37:25.490Z
updated_at: 2026-09-14T03:00:30.884Z
started_at: 2026-09-14T02:38:24.206Z
closed_at: 2026-09-14T03:00:30.882Z
close_reason: "Added Summary format and aggregation section: one softschema contract for session and aggregate summaries built from session extents with request-ID digests and optional request index; exact keyed-union aggregation rules, unresolved overlaps reported separately, mergeable histograms for non-additive measures; Pydantic authoring compiled to JSON Schema 2020-12 consumed by Rust."
resolution: null
duplicate_of: null
---
Define one well-specified softschema format (github.com/jlevy/softschema) for urollup summaries: the same schema serves an individual session summary and an aggregate rollup, and any number of summaries aggregate into another valid summary. Aggregation must be idempotent and must not double-count overlapping or repeated sessions, so summaries carry the session identities and extents they cover. Use the Python softschema tooling during development to author, compile and check the contract (compile --check in CI); the Rust implementation consumes the compiled JSON Schema. Reconcile with the observation-level bundle design (uro-2bxv) and document the rationale in the research brief's portable merging section.
