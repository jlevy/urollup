---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: in_progress
priority: 1
version: 10
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m2kswxt5gxe3s49wch9dss46
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2yrezrf530kbz15erhh7hw6
hold: null
hold_until: null
created_at: 2026-09-20T06:36:54.265Z
updated_at: 2026-09-28T04:14:40.538Z
started_at: 2026-09-28T04:05:43.795Z
---
R4 and cache contract: retain provider/billing-channel basis, exact model and served/requested basis, request timestamps, service tier/speed from recorded fields, inclusive input and cache lifetimes. Retain unknowns, conflicting lifetime diagnostics and reasoning as output subset. Report counted requests with positive/zero/unknown cache reads and writes; one request can read and write, not separate cache API calls. Expose token-read share and request-hit share with distinct observed populations, additive numerators/denominators and availability counts. No model-config inference for historical tier. Synthetic fixtures verify all fields survive reconciliation and cover missing/partial data; serialization ownership remains uro-ni7m/uro-vgea and pricing ownership uro-neii.

## Notes

Active on codex/alpha-pricing-context above green, review-ready PR16. Corrected Codex normalization subtracts recorded cache reads and writes from inclusive input; native mapping and official formula were verified. Nonzero-write red/green, combined-cache underflow and maximum-count tests added; workspace tests passed for that correction. Added shared optional PricingContext to observations and requests (provider, billing channel, tier, speed, geography, conflict marker), increasing row size limits by one 8-byte pointer slot. Reconciliation retains original context, ignores copies, fills missing original fields and diagnoses conflicting originals so the matcher can leave them unpriced. Two new context tests pass. Adapter producers, pricing matcher, report fields, cache populations, docs, full gates and private QA remain outstanding. No pricing PR or pricing acceptance yet.
