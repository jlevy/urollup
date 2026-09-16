---
type: is
id: is-01m2kt2ce2nta6ssw3qr0rw2zk
title: "Implement price matching: dates, tiers, context bands and cache-write durations"
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
dependencies:
  - type: blocks
    target: is-01m2kt2dgbm2s63dhvm4y3h2g9
  - type: blocks
    target: is-01m2kt2edxydx274wawh93bjxx
  - type: blocks
    target: is-01m2kt2f8c62j2vaw2k4swfaj6
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-16T00:32:34.237Z
updated_at: 2026-09-16T03:01:54.743Z
closed_at: 2026-09-16T03:01:54.742Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-neii is the original.
resolution: duplicate
duplicate_of: is-01m2kswxt5gxe3s49wch9dss46
---
Milestone 0.4: apply the price table to requests. Design §4.5 and §4.1 (Candidate, §9.1 Pricing Policy).

Acceptance:
- A request uses the rate in effect at its timestamp, matched on model, service tier, context band and cache-write duration; models match exactly or through listed aliases, never by fuzzy prefix.
- A context band applies to the whole request when its inclusive input (uncached input, cache reads and cache writes) exceeds the band's threshold, and the highest matching threshold wins.
- Reasoning tokens are priced within output, never twice.
- A dimension a dialect does not record uses the provider's documented default (standard tier, 5-minute cache writes) and those tokens are reported default-assumed; an observed but unrecognized value, and a matched row with no rate for a token category, leave them unpriced.
- Codex requests match their requested model; placeholder names such as codex-auto-review stay unpriced; service tier comes only from recorded fields (Codex thread_settings_applied), never from an agent's configuration files.
- List-price estimates stay a separate measure from source-reported cost, which is never substituted for them.
