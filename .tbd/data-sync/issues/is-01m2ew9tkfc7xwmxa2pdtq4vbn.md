---
type: is
id: is-01m2ew9tkfc7xwmxa2pdtq4vbn
title: "Plan: specify deterministic derivation of analytical IDs"
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe0xffh11dgc6acjznk3r
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:23.118Z
updated_at: 2026-09-14T02:49:51.512Z
started_at: 2026-09-14T02:38:24.097Z
closed_at: 2026-09-14T02:49:51.511Z
close_reason: "Added Analytical identities subsection: <prefix>-v<version>-<digest> IDs from SHA-256 over RFC 8785 canonical keys, per-prefix key precedence table, native/fallback/ambiguous basis, alias rule independent of merge order."
resolution: null
duplicate_of: null
---
The ledger section introduces typed IDs (src-, thr-, req-, act-, rpt-) under a versioned identity contract but not how they are derived. Idempotent, commutative merge across machines requires deterministic content/namespace-derived IDs, not random ones. Done when the plan specifies derivation inputs per entity (native namespace, provider, account, native IDs, fallbacks), hashing/encoding, versioning, and how weak-evidence derived identities are marked ambiguous.
