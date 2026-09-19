---
type: is
id: is-01m2xqdnqejxxt9dexr6218q7f
title: Measure a process allocator for whole-history RSS
kind: task
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-19T20:58:42.798Z
updated_at: 2026-09-19T20:58:42.798Z
---
Phase 2 leftover after field cuts. Grouping still holds ~612k 224 B shells; the remaining 141 MiB may be allocator slack rather than live rows. Try one explicit allocator (follow SUPPLY-CHAIN-SECURITY.md) on the urollup binary only, not urollup-core. Measure privacy-safe WH and Codex-only.

Files: crates/urollup/Cargo.toml and crates/urollup/src/main.rs (or a tiny alloc module). Close only if peak falls. Revert if peak or wall rises or if the crate fails the supply-chain gate.
