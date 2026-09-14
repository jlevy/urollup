---
type: is
id: is-01m2gttdzk1s48d87gzvmhfmt5
title: Add the capture cache read path
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
  - capture-cache
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T20:47:58.962Z
updated_at: 2026-09-14T20:47:58.962Z
---
Phase 2: read captured records for unchanged source prefixes and parse only appended complete records as new segments; default prefix check (identity, size, first-record and tail digests) with --verify-cache full-extent hashing; --no-cache and --rebuild-cache; cached, uncached and rebuilt golden equivalence in CI; benchmark against the spike baseline (explorations/log-throughput).
