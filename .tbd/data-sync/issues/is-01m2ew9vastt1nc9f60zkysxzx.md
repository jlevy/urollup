---
type: is
id: is-01m2ew9vastt1nc9f60zkysxzx
title: "Plan: choose the initial price data source for Phase 1"
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe0xffh11dgc6acjznk3r
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:23.864Z
updated_at: 2026-09-14T02:51:46.809Z
started_at: 2026-09-14T02:38:24.425Z
closed_at: 2026-09-14T02:51:46.808Z
close_reason: "Pricing: reviewed versioned table built into the binary from provider pages (LiteLLM/models.dev cross-checks only), exact model matching, assumed-default labeling, offline-only with --prices overrides, 90-day staleness warning; open question narrowed."
resolution: null
duplicate_of: null
---
Pricing source is an open question, yet Phase 1 reports include price estimates and pricing coverage. Done when the plan recommends a concrete initial price data source and update process (e.g. a reviewed, versioned bundled table, optionally seeded from a public dataset such as the one ccusage uses), its effective-date/tier/cache-duration fields, and narrows the open question to what remains undecided.
