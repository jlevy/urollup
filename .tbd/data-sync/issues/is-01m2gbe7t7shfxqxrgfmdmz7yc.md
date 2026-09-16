---
type: is
id: is-01m2gbe7t7shfxqxrgfmdmz7yc
title: Implement the durable capture store
kind: task
status: open
priority: 2
version: 12
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - capture-cache
  - phase-1
  - milestone-0.3
dependencies:
  - type: blocks
    target: is-01m2gttdzk1s48d87gzvmhfmt5
  - type: blocks
    target: is-01m2kegstn5bx58s7sc8d0s01x
  - type: blocks
    target: is-01m2kswehkpv3849n6hmp0knr6
  - type: blocks
    target: is-01m2kswhqgy6p4h2zt4nq0267p
  - type: blocks
    target: is-01m2kswpq22hpbr9b3cx7rbht3
  - type: blocks
    target: is-01m2kt2762jcwn6nbhdq0tssnk
  - type: blocks
    target: is-01m2kt28sfev3pw3wzxw8qxkap
  - type: blocks
    target: is-01m2kt29zwzgc17btptrez7yh6
parent_id: is-01m2ke4qtjw7gzyjmmzxxs64mf
created_at: 2026-09-14T16:19:10.790Z
updated_at: 2026-09-16T00:32:31.735Z
---
Durable owner-only capture store in the platform data directory (UROLLUP_CAPTURE_DIR override), on by default: versioned per-dialect strip policy; one entry per logical source keyed by src- ID with manifest and zstd JSONL segments; every run writes atomic replacement entries for new or changed sources; reads captured records for sources whose logs are gone (reported as retained) and keeps previous entries as retained versions when a source is rewritten in place; --no-capture, capture status and capture prune; NamedTempFile staging with fsync, digest verification, persist_noclobber and compare-and-swap manifest replacement under an OS advisory lock; golden equivalence with and without the store.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Hardening: OS advisory lock, owner-only staged files, fsync each segment and the entry directory before replacing the manifest, verify a segment digest before publishing, unique temp names, compare-and-swap manifest replacement.
- A path that vanishes mid-scan is a mid-scan change; a .gz or .zst sibling with the same identity is a representation change, not a new source; in-place migration keeps previously captured observations as evidence and diagnoses records missing afterwards.
- Document each digest's purpose; the default prefix check can miss a same-size mutation inside the captured extent, which --verify-cache detects.
- Spike: cache 340 MB for 19.5 GB of logs (zstd 3); ~1.4x faster for 30 days, ~1.9x for all history on 10 threads.
