---
type: is
id: is-01m2xdfybf0rn6n0b97jhdm8kt
title: Drop Claude intern HashMap after each source decodes
kind: task
status: closed
priority: 1
version: 7
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
created_at: 2026-09-19T18:05:11.405Z
updated_at: 2026-09-19T20:56:09.904Z
started_at: 2026-09-19T18:05:15.456Z
closed_at: 2026-09-19T20:56:09.904Z
close_reason: "Peak did not fall or the design was abandoned. Field and ID relocation are exhausted (uro-t8ws standing 653/586). Do not retry two-pass Claude, prefix-merge, boxing shells, dropping KeyGraph, one-inline-key, per-rollout shrink_to_fit, or chunked consume. Tail-consume after grouping was not filed: the grouping peak holds every shell before Requests are reserved, and shrink_to_fit of that remainder reallocs while the table is still live (already raised RSS)."
resolution: canceled
duplicate_of: null
---
uro-h0fw prefix-merge raised peak to 830-854 MiB and was reverted. After decode, intern is finished; absorb only walks the string vector. The HashMap on all 1775 DecodedSources waits on the join beside the Codex ledger. Add Strings::freeze (drop symbols, shrink the vector) at the end of decode_source. Files: claude_project.rs Strings::freeze, decode_source. Peak must fall.

## Notes

2026-09-19: Freeze kept. No quiet-machine measure this pass (load 83-133). Peak is still Codex (650 MiB Codex-only after uro-3y9m; Claude-only 357). Left open.
