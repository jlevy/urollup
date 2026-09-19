---
type: is
id: is-01m2xdfybf0rn6n0b97jhdm8kt
title: Drop Claude intern HashMap after each source decodes
kind: task
status: in_progress
priority: 1
version: 4
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
updated_at: 2026-09-19T18:09:10.857Z
started_at: 2026-09-19T18:05:15.456Z
---
uro-h0fw prefix-merge raised peak to 830-854 MiB and was reverted. After decode, intern is finished; absorb only walks the string vector. The HashMap on all 1775 DecodedSources waits on the join beside the Codex ledger. Add Strings::freeze (drop symbols, shrink the vector) at the end of decode_source. Files: claude_project.rs Strings::freeze, decode_source. Peak must fall.

## Notes

2026-09-19: Implemented Strings::freeze in decode_source. Fixture, snapshot and worker-count tests passed. sessions --all: 751 MiB (768720 KiB) / 22.8 s, then 830 MiB (849968 KiB) / 27.0 s, then 835 MiB (854656 KiB) / 45.5 s, load 58-128. No consistent fall vs uro-mxcp 796 MiB. Kept the freeze (intern map is unused after decode). Left open.
