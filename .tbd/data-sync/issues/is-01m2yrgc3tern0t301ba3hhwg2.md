---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: in_progress
priority: 1
version: 14
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
updated_at: 2026-10-09T07:16:39.961Z
started_at: 2026-09-28T04:05:43.795Z
---
R4 and cache contract: retain provider/billing-channel basis, exact model and served/requested basis, request timestamps, service tier/speed from recorded fields, inclusive input and cache lifetimes. Retain unknowns, conflicting lifetime diagnostics and reasoning as output subset. Report counted requests with positive/zero/unknown cache reads and writes; one request can read and write, not separate cache API calls. Expose token-read share and request-hit share with distinct observed populations, additive numerators/denominators and availability counts. No model-config inference for historical tier. Synthetic fixtures verify all fields survive reconciliation and cover missing/partial data; serialization ownership remains uro-ni7m/uro-vgea and pricing ownership uro-neii.

## Notes

Pricing foundation remains uncommitted on codex/alpha-pricing-context above review-ready PR16. Codex inclusive input now subtracts reads and writes. Original-only PricingContext reconciliation marks conflicts and ignores copy metadata. Codex captures recorded provider and nested service tier per thread/turn, isolates inherited settings, preserves unknown strings, and passes delayed-direct and cumulative-counter tests. Claude now retains usage.speed, service_tier and inference_geo without inferring provider/channel. Repeated native contexts share an Arc; missing per-request fields never inherit previous values. Claude decoded record allowance increased from 200 to 208 bytes. New Claude ingestion and missing-metadata/storage-sharing tests pass, as do generated parser/document equivalence cases. Full workspace tests and all-target all-feature clippy passed; evidence is external pricing-context-both-tests.log. Still required: distinguish malformed pricing dimensions from absent defaults before matcher; no-turn Codex context edge cases; cache-request coverage metrics; reviewed rates and exact matcher; report cost output; docs/full make check; process-wide memory safety, scale proofs, and accepted-head private QA. No pricing PR or alpha acceptance.


The parent of this bead is:
---
type: is
id: is-01m2yrezrf530kbz15erhh7hw6
title: "Spec: Clean whole-history usage analysis and list-price estimates"
kind: epic
status: open
priority: P1
version: 10
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2yrfm4816gxgm14jk3c0erq
  - is-01m2yrgbar7pmmncksknephs5s
  - is-01m2yrgc3tern0t301ba3hhwg2
  - is-01m2yrgcxaf3ewkzv9cjawjf18
  - is-01m2yrhe5crqpp22w45b3243ev
  - is-01m2yrherhddnx2j22g3xtqedr
  - is-01m3jwwpp0h7j5azt28c75zcsj
  - is-01m3jzhnnk1xk472z6y6gfj5be
  - is-01m3na5hvvbrsrgtrn2v6w4jye
created_at: 2026-09-20T06:36:08.846Z
updated_at: 2026-09-29T00:48:51.578Z
---
Deliver G5 in the linked plan: one reconciled snapshot, reusable artifacts, joint calendar/agent/provider/model grouping, cache metrics, tools and time, and explicit list-price estimates. Reuse existing summary/bundle and pricing work. Acceptance requires documented commands and a maintained runner without bespoke scripts. Full workflow remains milestone 0.5; this does not silently expand the 0.1 release scope.


2026-10-09: the uncommitted pricing work is preserved as draft PR https://github.com/jlevy/urollup/pull/19 (branch codex/alpha-pricing-context, commits 9a6f6c9 code and 18f5260 docs), the top layer of formal stack #17 above #16. Not ready for review; rebase onto #18 when it lands (shared adapter and diagnostics files).
