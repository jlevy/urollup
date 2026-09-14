---
type: is
id: is-01m2gbe7t7shfxqxrgfmdmz7yc
title: Implement the default-on capture cache
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - capture-cache
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T16:19:10.790Z
updated_at: 2026-09-14T19:58:19.610Z
---
Local idempotent capture cache of per-source captured records in the platform cache directory (UROLLUP_CACHE_DIR override), keyed by src- ID and versioned by adapter and strip policy; append segments for growing logs, prefix checks with regeneration, --no-cache, --rebuild-cache, --verify-cache, cache status and cache prune; NamedTempFile staging with persist/persist_noclobber and per-entry locks; cached, uncached and rebuilt golden equivalence in CI. Phase set by uro-1k0u.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Hardening: OS advisory lock, owner-only staged files, fsync each segment and the entry directory before replacing the manifest, verify a segment digest before publishing, unique temp names, compare-and-swap manifest replacement.
- A path that vanishes mid-scan is a mid-scan change; a .gz or .zst sibling with the same identity is a representation change, not a new source; in-place migration keeps previously captured observations as evidence and diagnoses records missing afterwards.
- Document each digest's purpose; the default prefix check can miss a same-size mutation inside the captured extent, which --verify-cache detects.
- Spike: cache 340 MB for 19.5 GB of logs (zstd 3); ~1.4x faster for 30 days, ~1.9x for all history on 10 threads.
