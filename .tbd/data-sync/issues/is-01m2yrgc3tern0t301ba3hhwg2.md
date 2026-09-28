---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: in_progress
priority: 1
version: 8
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
updated_at: 2026-09-28T04:08:30.919Z
started_at: 2026-09-28T04:05:43.795Z
---
R4 and cache contract: retain provider/billing-channel basis, exact model and served/requested basis, request timestamps, service tier/speed from recorded fields, inclusive input and cache lifetimes. Retain unknowns, conflicting lifetime diagnostics and reasoning as output subset. Report counted requests with positive/zero/unknown cache reads and writes; one request can read and write, not separate cache API calls. Expose token-read share and request-hit share with distinct observed populations, additive numerators/denominators and availability counts. No model-config inference for historical tier. Synthetic fixtures verify all fields survive reconciliation and cover missing/partial data; serialization ownership remains uro-ni7m/uro-vgea and pricing ownership uro-neii.

## Notes

Active implementation on codex/alpha-pricing-context, stacked above PR16 (3c4e9cd). Confirmed native mapping in openai/codex commit 6b9826e3aa83b1a5947db50f4332cb9c65f1b340 codex-rs/codex-api/src/sse/responses.rs: input_tokens passes through unchanged, cache_write_input_tokens maps from input_tokens_details.cache_write_tokens. Official prompt-caching pricing formula subtracts reads and writes from inclusive input: https://developers.openai.com/api/docs/guides/prompt-caching#monitor-cache-performance. Synthetic regression failed as expected (100 inclusive, 20 read, 10 write produced 80 ordinary rather than 70); corrected normalizer passes token unit tests. Added combined-cache underflow and maximum-count regression, full workspace tests running (exec session 23650 is not authoritative; inspect current tool handle recorded in conversation). Still need pricing context retention, request-cache metrics, CLI integration, docs and full gates. Preserve multi-model/advisor components and distinct-request populations. No private values are recorded here.
