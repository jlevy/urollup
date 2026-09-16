---
type: is
id: is-01m2kt2b1f9r9affgkx4z03x76
title: Add capture equivalence goldens across store modes
kind: task
status: closed
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.3
  - capture-cache
dependencies: []
parent_id: is-01m2ke4qtjw7gzyjmmzxxs64mf
created_at: 2026-09-16T00:32:32.812Z
updated_at: 2026-09-16T03:01:54.488Z
closed_at: 2026-09-16T03:01:54.487Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-zdih is the original.
resolution: duplicate
duplicate_of: is-01m2kswtgsj4hcyk1x890reavj
---
Milestone 0.3: prove the store changes no result before capture ships on by default. Design §2.5 (Equivalence) and the plan's Capture and privacy tests.

Acceptance:
- CI runs the golden suite with the store disabled, with a populated store and with retained sources, and every mode produces identical ledgers and reports for the same snapshot and policy.
- Repeating a run over the same sources never adds usage (idempotence).
- A leak fixture whose record carries text under an unknown key yields no plaintext in the summary, bundle or records table, while the capture store keeps the value and reports the unknown key.
- This suite is the baseline the Phase 2 cache read path and --rebuild-cache must also satisfy (uro-2y6v).
