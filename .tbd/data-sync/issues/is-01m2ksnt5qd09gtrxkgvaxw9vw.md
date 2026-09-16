---
type: is
id: is-01m2ksnt5qd09gtrxkgvaxw9vw
title: Reconcile thread, relationship, tool action and provider limit entities
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:42.326Z
updated_at: 2026-09-16T08:04:18.656Z
closed_at: 2026-09-16T08:04:18.655Z
close_reason: "Implemented in a50585b: ReconcileInput and Ledger now cover threads, relationships, tool actions, and provider limit observations; reconciliation validates and canonicalizes identities and aliases, merges evidence, diagnoses conflicting properties, preserves relationship confidence, and collapses only consecutive identical limit snapshots. Claude and Codex adapters route these entities through the ledger. Added deterministic and property tests for alias references, order independence, and repeated-import idempotence. CARGO_INCREMENTAL=0 make check passes."
resolution: null
duplicate_of: null
---
uro-spce defines every normalized ledger entity from design 3.1 and reconciles request observations into logical requests, but the engine's input and output cover requests only. The adapters in uro-y2qj will also emit thread, relationship, tool action and provider limit observations, which need the same treatment: dedupe by identity, merge evidence, diagnose conflicting properties rather than picking one, and keep the result independent of traversal order. Codex repeats an identical rate_limits snapshot in every token_count event, so identical consecutive limit observations must collapse into one.

Extend ReconcileInput and Ledger in crates/urollup-core/src/ledger/reconcile.rs with those entity kinds, and add the order-independence and idempotence property tests the request path already has.
