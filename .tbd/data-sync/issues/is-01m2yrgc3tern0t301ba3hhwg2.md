---
type: is
id: is-01m2yrgc3tern0t301ba3hhwg2
title: Retain pricing context and expose cache-request metrics with coverage
kind: feature
status: in_progress
priority: 1
version: 20
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
child_order_hints:
  - is-01m4paeyrbe8wbe0cy4v6wy4fb
hold: null
hold_until: null
created_at: 2026-09-20T06:36:54.265Z
updated_at: 2026-10-11T21:24:57.384Z
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


2026-10-09 (restack): draft PR https://github.com/jlevy/urollup/pull/19 is rebased onto main bdaa7ce (PRs #15, #16, #18 and #22 merged). New head 95ae000 (code commit 0a102d8, docs commit 95ae000), base main, still a draft and not ready for review. Main's accounting rules, diagnostics, coverage and tests are unchanged apart from the intended Codex cache-write subtraction; pricing context follows main's turn_id context lookup, and docs/formats now describe main. Local fmt, clippy -D warnings, rustdoc, 347 workspace tests, make golden and 32 e2e result cases pass with no golden change (every Codex fixture records zero cache writes). Restack follow-ups are in the PR description: the service-tier filter versus main's unplaced (missing-ordinal) settings events, the 200-to-208-byte Claude record budget comment and its memory accounting, the AdapterError::Input doc comment, and no fixture with nonzero Codex cache writes.

2026-10-11, PR #19 round 2 (head after round-2 commits a665e64): Codex usage whose input is below cache reads plus writes is now excluded per record or counter step with codex-inclusive-input-underflow and an InconsistentUsage coverage gap, not an abort (review B1/B2). Codex thread_settings_applied is read as a complete snapshot: a missing service_tier clears the tier and model_provider_id updates the provider; the tier carries TierBasis::ThreadSetting (Codex, the thread's selected tier, which a request can differ from) or TierBasis::Request (Claude); how a matcher treats ThreadSetting is for uro-wuby. PricingContext is interned (&'static, like Name). Landing order (review A11): PR #29 (admission cost model, uro-6pi8) merges first; then rebase PR #19 onto it, update kappa_is_pinned_to_the_row_sizes and the plan's kappa table for RequestObservation 232 B, Request 224 B and the Claude decoded record 3 x 184 = 552 B, and charge the interned PricingContext as a process intern in DeepSize and the model. Still open here: malformed versus absent pricing fields, Codex usage without a turn, cache-request metrics.

2026-10-11: PR #19 merged (squash 11b6eec) after reviews A-E: Codex cache writes counted once, inconsistent Codex usage degraded to a gap with codex-inclusive-input-underflow, each request's pricing context retained and interned, and its intern, the pricing merge's group scratch and the coordinating thread's caches charged in the admission model. Pricing-context retention is done; this bead stays open for cache-request metrics with coverage, malformed versus absent pricing fields and Codex usage without a turn. Child uro-1uy7: thread-less settings snapshots never set a migrated (unmarked) fork's tier.
