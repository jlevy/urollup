---
type: is
id: is-01m2ksyv7sj854y8xb6gh0sqyg
title: Define cache keys and versioned boundaries for layers 2 and 3
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-3
dependencies:
  - type: blocks
    target: is-01m2f0tk5g8r3bxm5s2r4mfajg
parent_id: is-01m2ksyqjx3eag5b7j4mbq4104
created_at: 2026-09-16T00:30:38.319Z
updated_at: 2026-09-16T00:32:04.059Z
---
Phase 3: the versioned boundaries the design fixes now, before storage is chosen. Design §8.3 (Ledger and Query Cache).

Acceptance:
- Observations are keyed by source content revision and adapter and schema version; reconciliation by observation identities and policy version; prices and queries separately, including filters, timezone, identity mapping and coverage policy, so a price update never reparses logs.
- Key definitions are documented and versioned alongside the contracts, and a version bump invalidates only the affected layer.
- Pricing stays independently versioned, and retained snapshots stay explicitly historical.
- The key design is reviewed before the storage choice (uro-sgeq), because it constrains what the store must support.
