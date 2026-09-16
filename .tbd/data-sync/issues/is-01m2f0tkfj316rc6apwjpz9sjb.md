---
type: is
id: is-01m2f0tkfj316rc6apwjpz9sjb
title: Handle append, replacement, deletion and late updates in cache
kind: task
status: open
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-3
dependencies:
  - type: blocks
    target: is-01m2f0tm43qfyq1hn5s796yc42
parent_id: is-01m2ksyqjx3eag5b7j4mbq4104
created_at: 2026-09-14T03:54:27.186Z
updated_at: 2026-09-16T00:31:36.857Z
---
Incremental cache maintenance with independently versioned pricing.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Hardening: OS advisory lock, owner-only staged files, fsync each segment and the entry directory before replacing the manifest, verify a segment digest before publishing, unique temp names, compare-and-swap manifest replacement.
- A path that vanishes mid-scan is a mid-scan change; a .gz or .zst sibling with the same identity is a representation change, not a new source; in-place migration keeps previously captured observations as evidence and diagnoses records missing afterwards.
- Document each digest's purpose; the default prefix check can miss a same-size mutation inside the captured extent, which --verify-cache detects.
- Spike: cache 340 MB for 19.5 GB of logs (zstd 3); ~1.4x faster for 30 days, ~1.9x for all history on 10 threads.
