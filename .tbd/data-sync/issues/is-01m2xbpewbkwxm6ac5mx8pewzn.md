---
type: is
id: is-01m2xbpewbkwxm6ac5mx8pewzn
title: Inline Request evidence for the common one-or-two-record case
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
hold: null
hold_until: null
created_at: 2026-09-19T17:33:47.786Z
updated_at: 2026-09-19T20:56:09.892Z
started_at: 2026-09-19T17:34:03.748Z
closed_at: 2026-09-19T20:56:09.891Z
close_reason: "Peak did not fall or the design was abandoned. Field and ID relocation are exhausted (uro-t8ws standing 653/586). Do not retry two-pass Claude, prefix-merge, boxing shells, dropping KeyGraph, one-inline-key, per-rollout shrink_to_fit, or chunked consume. Tail-consume after grouping was not filed: the grouping peak holds every shell before Requests are reserved, and shrink_to_fit of that remainder reallocs while the table is still live (already raised RSS)."
resolution: canceled
duplicate_of: null
---
After uro-l3fw's two-pass Claude re-decode raised whole-history peak to 929-946 MiB and was reverted, sessions --all is 804 MiB (823632 KiB) / 22.1 s. Codex ingest finishes first; release_discovery drops threads and sources, but Ledger.requests stays resident while Claude decodes. 611,314 Request rows each hold records: Box<[EvidenceRef]>. Most requests have one original and no copies, so this is one small heap allocation per request beside the Claude working set.

Replace Request.records with an inline 1-2 ref representation (InlineList<EvidenceRef, 1> or a dedicated small-vec), keep Request at or near its 240-byte budget, and update Request::evidence / copies / split_records plus the reconcile builder in reconcile.rs (the originals.iter().chain(copies).map(evidence).collect() site).

Acceptance: fixture snapshots and worker-count identity unchanged; Request size assert updated only if the layout requires it; make test; privacy-safe sessions --all on uro-l0gd. Peak must fall.

## Notes

2026-09-19: Implemented RecordRefs (Empty/One/Many) on Request.records; Request size assert 240→248. Fixture ledger/adapter/snapshot tests and clippy -D warnings passed. Privacy-safe sessions --all under load 47–53: RecordRefs 827–852 MiB (847040–872240 KiB) / 31.7–33.6 s; same-machine Box<[EvidenceRef]> A/B 840 MiB (859808 KiB) / 30.1 s. No clear fall vs the quiet-machine 804 MiB / 22.1 s baseline; A/B is in the noise. Left open. Next leftover: Claude reconcile_input preallocates the full observation Vec while every ParsedRecord chunk is still alive.
