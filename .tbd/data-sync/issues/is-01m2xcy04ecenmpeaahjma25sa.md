---
type: is
id: is-01m2xcy04ecenmpeaahjma25sa
title: Merge Claude DecodedSources as the discovery-order prefix completes
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
created_at: 2026-09-19T17:55:23.397Z
updated_at: 2026-09-19T20:56:09.948Z
started_at: 2026-09-19T17:56:19.836Z
closed_at: 2026-09-19T20:56:09.948Z
close_reason: "Peak did not fall or the design was abandoned. Field and ID relocation are exhausted (uro-t8ws standing 653/586). Do not retry two-pass Claude, prefix-merge, boxing shells, dropping KeyGraph, one-inline-key, per-rollout shrink_to_fit, or chunked consume. Tail-consume after grouping was not filed: the grouping peak holds every shell before Requests are reserved, and shrink_to_fit of that remainder reallocs while the table is still live (already raised RSS)."
resolution: canceled
duplicate_of: null
---
After uro-mxcp, loaded whole-history peak is 796 MiB (815,008 KiB). Claude still calls try_read_in_parallel, which retains every DecodedSource (per-source Strings intern table plus Vec<ParsedRecord>) until Corpus::merge absorbs them in discovery order. That join vec sits beside the Codex ledger (611,314 Request rows) for the whole Claude ingest.

Deliver completed sources into a slot array and merge source i as soon as 0..i are present, so earlier intern tables and record vectors drop before later workers finish. Keep discovery-order absorb so worker-count identity holds. Do not re-decode (uro-l3fw already raised peak). try_read_in_parallel_for_each stays for tests; this needs prefix-order merge, not completion-order consume.

Files: sources/parallel.rs (prefix-ready helper); claude_project.rs ingest_discovery_with_workers and Corpus::merge.

Acceptance: fixture snapshots and worker-count identity unchanged; make test; privacy-safe sessions --all on uro-l0gd. Peak must fall.

## Notes

2026-09-19: Prefix merge implemented then reverted. sessions --all 854 MiB (874640 KiB) / 36.2 s and 830 MiB (849824 KiB) / 30.4 s at load 52-73, above uro-mxcp 796 MiB. In-flight decode overlapped the growing corpus. Claude again uses try_read_in_parallel + Corpus::merge. Left open.
