---
type: is
id: is-01m2xqdnqejxxt9dexr6218q7f
title: Measure a process allocator for whole-history RSS
kind: task
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
  - memory
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-19T20:58:42.798Z
updated_at: 2026-09-19T21:07:02.403Z
started_at: 2026-09-19T21:03:18.263Z
closed_at: 2026-09-19T21:07:02.401Z
close_reason: Process mimalloc 0.1.52 on the urollup binary only. Quiet WH 834 MiB (853568 KiB) / 19.0 s vs standing 653 / 17.3; Codex-only 676 MiB (692016 KiB) / 15.3 s vs 586 / 13.2. Peak and wall both rose. Reverted. Supply-chain and dependency-guard passed while it was in tree.
resolution: canceled
duplicate_of: null
---
Phase 2 leftover after field cuts. Grouping still holds ~612k 224 B shells; the remaining 141 MiB may be allocator slack rather than live rows. Try one explicit allocator (follow SUPPLY-CHAIN-SECURITY.md) on the urollup binary only, not urollup-core. Measure privacy-safe WH and Codex-only.

Files: crates/urollup/Cargo.toml and crates/urollup/src/main.rs (or a tiny alloc module). Close only if peak falls. Revert if peak or wall rises or if the crate fails the supply-chain gate.
