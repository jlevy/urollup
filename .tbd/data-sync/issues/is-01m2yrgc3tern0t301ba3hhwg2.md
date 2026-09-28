---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: in_progress
priority: 1
version: 12
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
updated_at: 2026-09-28T04:26:26.558Z
started_at: 2026-09-28T04:05:43.795Z
---
R4 and cache contract: retain provider/billing-channel basis, exact model and served/requested basis, request timestamps, service tier/speed from recorded fields, inclusive input and cache lifetimes. Retain unknowns, conflicting lifetime diagnostics and reasoning as output subset. Report counted requests with positive/zero/unknown cache reads and writes; one request can read and write, not separate cache API calls. Expose token-read share and request-hit share with distinct observed populations, additive numerators/denominators and availability counts. No model-config inference for historical tier. Synthetic fixtures verify all fields survive reconciliation and cover missing/partial data; serialization ownership remains uro-ni7m/uro-vgea and pricing ownership uro-neii.

## Notes

Active on codex/alpha-pricing-context above review-ready PR16 (15 hosted checks green). Corrected Codex inclusive input normalization to subtract cache reads and writes, verified against native mapping and official formula. Shared optional PricingContext and original-only reconciliation explicitly diagnose context conflicts; row size guards account for the added 8-byte pointer slot. Codex now decodes model_provider and nested thread_settings.service_tier, snapshots per-thread context by turn, preserves unknown tier strings and excludes child-named inherited settings from child pricing. Direct delayed-record and inherited-fork tests observed red then passed. The counter integration test proves cache writes count once, missing settings keys preserve prior tier and unknown tier strings remain explicit. Full workspace tests passed after adapter changes; final all-target all-feature clippy passed and all three pricing-context integration tests passed. Claude producers, no-turn context edge cases, pricing matcher/table, report costs and coverage, docs, full make check, scale proofs and accepted-head private QA remain outstanding. No pricing PR or alpha acceptance yet.
