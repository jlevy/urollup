---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: in_progress
priority: 1
version: 7
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
updated_at: 2026-09-28T04:05:43.797Z
started_at: 2026-09-28T04:05:43.795Z
---
R4 and cache contract: retain provider/billing-channel basis, exact model and served/requested basis, request timestamps, service tier/speed from recorded fields, inclusive input and cache lifetimes. Retain unknowns, conflicting lifetime diagnostics and reasoning as output subset. Report counted requests with positive/zero/unknown cache reads and writes; one request can read and write, not separate cache API calls. Expose token-read share and request-hit share with distinct observed populations, additive numerators/denominators and availability counts. No model-config inference for historical tier. Synthetic fixtures verify all fields survive reconciliation and cover missing/partial data; serialization ownership remains uro-ni7m/uro-vgea and pricing ownership uro-neii.

## Notes

Preserve multi-model/advisor usage components for pricing at each component model rate. Cache-hit request populations retain distinct-request scope and cannot be reconstructed by summing overlapping per-model counts. Alpha pricing review: verify Codex input normalization before applying rates. Current normalize_input subtracts cache reads but then adds cache writes as a disjoint category; official Responses usage pricing subtracts both cached and cache-write tokens from inclusive input. Primary source: https://developers.openai.com/api/docs/guides/prompt-caching#monitor-cache-performance (fetched 2026-09-28). Confirm native Codex cache_write_input_tokens mapping, add a nonzero-write synthetic regression, and correct normalization if the mapping preserves inclusive input. No private history values informed this finding. Do not mark cost rollups accepted until date/tier/context/provider assumptions are explicit.
