---
type: is
id: is-01m5nfyz44g6tc2p07r7y1bej4
title: Canonicalize gap threads and diagnostic subjects through thread aliases
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - correctness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:30.883Z
updated_at: 2026-10-23T23:01:30.883Z
---
Latent. reconcile.rs:689-691 canonicalizes request owners and relationships through thread aliases but not gaps or diagnostics, which totals.rs:214-217 compares against selections, so a gap filed against an alias would not mark the selected canonical thread partial. Neither adapter creates aliases today (claude_project.rs:1490, codex_rollout.rs:2589). Fix before aliases arrive.
