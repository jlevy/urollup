---
type: is
id: is-01m2x8apr4dv9jctc0j20g6mbq
title: Cut whole-history wall time to 10 seconds
kind: task
status: in_progress
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-19T16:34:56.899Z
updated_at: 2026-09-19T21:19:49.019Z
started_at: 2026-09-19T21:07:49.244Z
---
Phase 2 speed target after the 512 MiB peak is met. Whole-history sessions --all is 22.3 s after uro-ecol on the reference laptop (Apple M1 Pro). Phase 2 acceptance is at most 10 s with outputs byte-identical to Phase 1.

Do not start until uro-n1cp closes. Measure first; then change only what the profile names.

Likely files once measured:
- crates/urollup-core/src/sources/prefilter.rs and adapters/codex_rollout.rs (memmem / RELEVANT_TYPE_TOKENS)
- crates/urollup-core/src/sources/parallel.rs (worker scheduling)
- crates/urollup-core/src/adapters/claude_project.rs and adapters/codex_rollout.rs (remaining decode cost after typed bodies)
- crates/urollup/src/cli.rs (ingest order, UROLLUP_STATS phases)

Acceptance: release sessions --all on the maintainer corpus at most 10 s; one and eight workers identical JSON; scale gates still green; privacy-safe numbers in the spec.

## Notes

2026-09-19: Unblocked from uro-n1cp. Standing wall 17.3 s.

2026-09-19 profile (privacy-safe). Release quiet remasure: WH 648 MiB / 18.2 s; Codex-only 573 MiB / 14.3 s. Standing band 653 / 17.3 and 586 / 13.2. The ~7 s over 10 s is Codex worker decode (kernel read ~40%, Line::read ~12%, memmem ~10%). Grouping is negligible.

2026-09-19: uro-s5vb implemented a zero-copy accept that matched parse_record in tests, then reverted: quiet WH 650 / 20.7 and Codex-only 583 / 15.3. Do not retry. Remaining 10 s work is not another validate_record replacement.
