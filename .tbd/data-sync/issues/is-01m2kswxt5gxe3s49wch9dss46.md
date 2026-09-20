---
type: is
id: is-01m2kswxt5gxe3s49wch9dss46
title: "Implement price matching: dates, tiers, context bands and cache-write durations"
kind: task
status: open
priority: 1
version: 6
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
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-16T00:29:35.415Z
updated_at: 2026-09-20T06:37:56.790Z
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

Whole-history workflow plan plan-2026-09-20-usage-analysis-workflow.md adds explicit historical-at-request-date versus chosen-price-date counterfactual valuation. Preserve model/provider/channel basis, service tier/speed, context band and cache lifetimes before aggregation; missing usage has unknown monetary value. Price each request or a lossless pricing partition with exact decimals. Report priced/default-assumed/unpriced populations and unknown coverage; do not equate list price with subscriptions. Input evidence is owned by uro-381f; G5 uro-i6xb verifies the workflow. Existing Candidate pricing policy is not silently accepted.
