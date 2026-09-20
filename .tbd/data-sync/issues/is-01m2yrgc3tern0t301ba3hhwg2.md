---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: open
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m2kswxt5gxe3s49wch9dss46
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-20T06:36:54.265Z
updated_at: 2026-09-20T06:46:53.124Z
---
R4 and cache contract: retain provider/billing-channel basis, exact model and served/requested basis, request timestamps, service tier/speed from recorded fields, inclusive input and cache lifetimes. Retain unknowns, conflicting lifetime diagnostics and reasoning as output subset. Report counted requests with positive/zero/unknown cache reads and writes; one request can read and write, not separate cache API calls. Expose token-read share and request-hit share with distinct observed populations, additive numerators/denominators and availability counts. No model-config inference for historical tier. Synthetic fixtures verify all fields survive reconciliation and cover missing/partial data; serialization ownership remains uro-ni7m/uro-vgea and pricing ownership uro-neii.

## Notes

Preserve multi-model/advisor usage components for pricing at each component model rate. Cache-hit request populations must retain their declared distinct-request scope and must not be reconstructed by summing overlapping per-model request counts.
