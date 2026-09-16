---
type: is
id: is-01m2kswtgsj4hcyk1x890reavj
title: Add capture equivalence goldens across store modes
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.3
  - capture-cache
dependencies:
  - type: blocks
    target: is-01m2gttdzk1s48d87gzvmhfmt5
parent_id: is-01m2ke4qtjw7gzyjmmzxxs64mf
created_at: 2026-09-16T00:29:32.056Z
updated_at: 2026-09-16T00:31:55.388Z
---
Milestone 0.3: prove the store changes no result before capture ships on by default. Design §2.5 (Equivalence) and the plan's Capture and privacy tests.

Acceptance:
- CI runs the golden suite with the store disabled, with a populated store and with retained sources, and every mode produces identical ledgers and reports for the same snapshot and policy.
- Repeating a run over the same sources never adds usage (idempotence).
- A leak fixture whose record carries text under an unknown key yields no plaintext in the summary, bundle or records table, while the capture store keeps the value and reports the unknown key.
- This suite is the baseline the Phase 2 cache read path and --rebuild-cache must also satisfy (uro-2y6v).
