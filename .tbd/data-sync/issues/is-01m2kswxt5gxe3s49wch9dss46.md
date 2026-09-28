---
type: is
id: is-01m2kswxt5gxe3s49wch9dss46
title: "Implement price matching: dates, tiers, context bands and cache-write durations"
kind: task
status: in_progress
priority: 1
version: 11
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
dependencies:
  - type: blocks
    target: is-01m2ksx04swwn45tchzcjqhr0g
  - type: blocks
    target: is-01m2ksx3pmqbfk86av545tdvab
  - type: blocks
    target: is-01m2ksx6f9w2rep42kwatz76k5
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-16T00:29:35.415Z
updated_at: 2026-09-28T07:41:45.984Z
---
Milestone 0.4: apply the price table to requests. Design §4.5 and §4.1 (Candidate, §9.1 Pricing Policy).

Acceptance:
- A request uses the rate in effect at its timestamp, matched on model, service tier, context band and cache-write duration; models match exactly or through listed aliases, never by fuzzy prefix.
- A context band applies to the whole request when its inclusive input (uncached input, cache reads and cache writes) exceeds the band's threshold, and the highest matching threshold wins.
- Reasoning tokens are priced within output, never twice.
- A dimension a dialect does not record uses the provider's documented default (standard tier, 5-minute cache writes) and those tokens are reported default-assumed; an observed but unrecognized value, and a matched row with no rate for a token category, leave them unpriced.
- Codex requests match their requested model; placeholder names such as codex-auto-review stay unpriced; service tier comes only from recorded fields (Codex thread_settings_applied), never from an agent's configuration files.
- List-price estimates stay a separate measure from source-reported cost, which is never substituted for them.

## Notes

Current uncommitted pricing foundation on codex/alpha-pricing-context: exact Decimal/string-only serde, disjoint token category pricing and unknown coverage, validated offline exact-key matching with dates/context bands, request-level model components/default labels/counterfactual date, and currency-separated additive cost totals. Synthetic tests cover advisor usage priced once, custom providers unmatched, missing dates/usage, context conflicts, overlap rejection, missing rates, reasoning subset and arithmetic overflow. On 2026-09-28 the full current cargo test --workspace run passed (external results/pricing-current-workspace-tests.log); all-target all-feature clippy also passed on this code. PR16 remains review-ready with all 15 hosted checks green, verified live. Pricing is still uncommitted and has no PR. Remaining: reviewed real rate table/provenance and override parser, documented default-policy data and cache-lifetime assumption labels, malformed dimensions and no-turn Codex cases, source-reported costs and CLI report integration, cache metrics, full make check, process-wide memory safety/scale proofs and accepted-head private end-to-end QA. Do not claim alpha readiness or full cost rollups yet.
