---
type: is
id: is-01m2xdfybf0rn6n0b97jhdm8kt
title: Drop Claude intern HashMap after each source decodes
kind: task
status: in_progress
priority: 1
version: 6
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
updated_at: 2026-09-19T18:33:22.443Z
started_at: 2026-09-19T18:05:15.456Z
---
uro-h0fw prefix-merge raised peak to 830-854 MiB and was reverted. After decode, intern is finished; absorb only walks the string vector. The HashMap on all 1775 DecodedSources waits on the join beside the Codex ledger. Add Strings::freeze (drop symbols, shrink the vector) at the end of decode_source. Files: claude_project.rs Strings::freeze, decode_source. Peak must fall.

## Notes

2026-09-19: Freeze kept. No quiet-machine measure this pass (load 83-133). Peak is still Codex (650 MiB Codex-only after uro-3y9m; Claude-only 357). Left open.
