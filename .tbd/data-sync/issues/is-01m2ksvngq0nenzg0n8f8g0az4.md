---
type: is
id: is-01m2ksvngq0nenzg0n8f8g0az4
title: Implement the portable-YAML reader and typed serde validators
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2ksvq7csxd3xhwb4zvcgnaj
  - type: blocks
    target: is-01m2ksvvwqgtym64mz080m0n6m
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:28:54.161Z
updated_at: 2026-09-16T00:29:00.693Z
---
Milestone 0.2: the Rust read and write path for artifacts, agreeing with softschema on every fixture. Design §5.7 and §8.4.

Acceptance:
- Typed serde structs with deny_unknown_fields mirror every compiled contract, so the run-time read path needs no Draft 2020-12 validator crate.
- The YAML reader enforces softschema portable value rules on parser events, not on built values: it rejects duplicate keys, anchors, aliases, merge keys, tags, non-string keys and integers beyond ±2^53, keeps date-shaped scalars as strings and bounds nesting depth. Tests run softschema 0.8.1's shared hardening vectors.
- Money is an exact decimal string and timestamps are checked against the contract pattern, because JSON Schema `format` is only an annotation.
- The compiled JSON Schema validator runs in `urollup validate` and in tests only.
- Cross-field rules live in Rust, since Pydantic validators are outside the compiled schema and its digest: digests match indexes, totals match extents, no request is counted in two extents, rows are sorted.
- Tests require the serde verdict, the compiled JSON Schema verdict and softschema's verdict to agree on every valid and invalid fixture, and require golden summaries and manifests written by urollup to pass `softschema validate` and `softschema repair --check` unchanged.
