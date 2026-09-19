---
type: is
id: is-01m2xdfybf0rn6n0b97jhdm8kt
title: Drop Claude intern HashMap after each source decodes
kind: task
status: in_progress
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
created_at: 2026-09-19T18:05:11.405Z
updated_at: 2026-09-19T18:14:00.834Z
started_at: 2026-09-19T18:05:15.456Z
---
uro-h0fw prefix-merge raised peak to 830-854 MiB and was reverted. After decode, intern is finished; absorb only walks the string vector. The HashMap on all 1775 DecodedSources waits on the join beside the Codex ledger. Add Strings::freeze (drop symbols, shrink the vector) at the end of decode_source. Files: claude_project.rs Strings::freeze, decode_source. Peak must fall.

## Notes

2026-09-19: Dialect-split (load 126-140): Codex-only 760 MiB (760224 KiB) / 44.2 s, Claude-only 357 MiB (365696 KiB) / 13.5 s. Whole-history peak is Codex ingest, so Strings::freeze cannot close this bead against 796 MiB. Left open.
