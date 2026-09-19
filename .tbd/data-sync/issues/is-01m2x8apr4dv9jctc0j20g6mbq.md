---
type: is
id: is-01m2x8apr4dv9jctc0j20g6mbq
title: Cut whole-history wall time to 10 seconds
kind: task
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-19T16:34:56.899Z
updated_at: 2026-09-19T16:34:56.899Z
---
Phase 2 speed target after the 512 MiB peak is met. Whole-history sessions --all is 22.3 s after uro-ecol on the reference laptop (Apple M1 Pro). Phase 2 acceptance is at most 10 s with outputs byte-identical to Phase 1.

Do not start until uro-n1cp closes. Measure first; then change only what the profile names.

Likely files once measured:
- crates/urollup-core/src/sources/prefilter.rs and adapters/codex_rollout.rs (memmem / RELEVANT_TYPE_TOKENS)
- crates/urollup-core/src/sources/parallel.rs (worker scheduling)
- crates/urollup-core/src/adapters/claude_project.rs and adapters/codex_rollout.rs (remaining decode cost after typed bodies)
- crates/urollup/src/cli.rs (ingest order, UROLLUP_STATS phases)

Acceptance: release sessions --all on the maintainer corpus at most 10 s; one and eight workers identical JSON; scale gates still green; privacy-safe numbers in the spec.
