---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: in_progress
priority: 1
version: 13
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
updated_at: 2026-09-28T04:30:57.185Z
started_at: 2026-09-28T04:05:43.795Z
---
R4 and cache contract: retain provider/billing-channel basis, exact model and served/requested basis, request timestamps, service tier/speed from recorded fields, inclusive input and cache lifetimes. Retain unknowns, conflicting lifetime diagnostics and reasoning as output subset. Report counted requests with positive/zero/unknown cache reads and writes; one request can read and write, not separate cache API calls. Expose token-read share and request-hit share with distinct observed populations, additive numerators/denominators and availability counts. No model-config inference for historical tier. Synthetic fixtures verify all fields survive reconciliation and cover missing/partial data; serialization ownership remains uro-ni7m/uro-vgea and pricing ownership uro-neii.

## Notes

Pricing foundation remains uncommitted on codex/alpha-pricing-context above review-ready PR16. Codex inclusive input now subtracts reads and writes. Original-only PricingContext reconciliation marks conflicts and ignores copy metadata. Codex captures recorded provider and nested service tier per thread/turn, isolates inherited settings, preserves unknown strings, and passes delayed-direct and cumulative-counter tests. Claude now retains usage.speed, service_tier and inference_geo without inferring provider/channel. Repeated native contexts share an Arc; missing per-request fields never inherit previous values. Claude decoded record allowance increased from 200 to 208 bytes. New Claude ingestion and missing-metadata/storage-sharing tests pass, as do generated parser/document equivalence cases. Full workspace tests and all-target all-feature clippy passed; evidence is external pricing-context-both-tests.log. Still required: distinguish malformed pricing dimensions from absent defaults before matcher; no-turn Codex context edge cases; cache-request coverage metrics; reviewed rates and exact matcher; report cost output; docs/full make check; process-wide memory safety, scale proofs, and accepted-head private QA. No pricing PR or alpha acceptance.
