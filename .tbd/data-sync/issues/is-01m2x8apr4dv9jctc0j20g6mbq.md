---
type: is
id: is-01m2x8apr4dv9jctc0j20g6mbq
title: Cut whole-history wall time to 10 seconds
kind: task
status: in_progress
priority: 1
version: 4
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
updated_at: 2026-09-19T21:14:03.929Z
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

2026-09-19 profile (privacy-safe; no paths, IDs, or prompt text). Release quiet remasure: WH 648 MiB (663248 KiB) / 18.2 s; Codex-only 573 MiB (586464 KiB) / 14.3 s. Standing band remains 653 / 17.3 and 586 / 13.2.

WH phases: discovery 0.43 s, codex_ingest 13.5 s (74%), claude_ingest 4.0 s (22%), session_index 0.06 s, query_render 0.15 s. Codex-only is almost all codex_ingest (13.9 s). Zeroing Claude still leaves ~14 s. The ~7 s over 10 s lives in Codex worker decode.

Sampling profile (profiling binary, Codex-only, 8 workers): main thread ~88% in pthread_join. Per worker: ~40% kernel read (reader.rs fill_buf; BufReader already 128 KiB), ~27% SourceDecoder::decode -> validate_record on may_be_relevant misses (serde_json deserialize_any walk of content lines), ~12% Line::read of relevant lines, ~10% memmem prefilter. reconcile.rs / grouping is negligible. After join, observe_parsed_source / IdentityKey::canonical_json_for is a small main-thread tail.

RSS: Codex-only is already 61-74 MiB over 512. 612561 observation shells * 224 B = 131 MiB; 611314 requests * 216 B = 126 MiB. The leftover is the live Codex ledger/intern/graph set, not Claude Value::from and not process-allocator slack (uro-96vw reverted).

Did not implement a cut this turn: skipping validate_record changes the malformed-line contract (design §2.2); nzo1 is the canceled-class number-without-Value pattern. Filed uro-s5vb for the prefilter-miss JSON walk. 10 s not met; this bead stays open.
