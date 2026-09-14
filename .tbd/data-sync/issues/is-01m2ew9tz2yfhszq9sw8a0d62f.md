---
type: is
id: is-01m2ew9tz2yfhszq9sw8a0d62f
title: "Plan: define how ambiguous-ownership requests count in default totals"
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
created_at: 2026-09-14T02:35:23.489Z
updated_at: 2026-09-14T02:49:51.928Z
started_at: 2026-09-14T02:38:24.121Z
closed_at: 2026-09-14T02:49:51.927Z
close_reason: Default totals now count owned, ambiguous and unknown requests once; ambiguous grouping, unresolved duplicates, possible measure for partial scope selection, and JSON ownership fields defined.
resolution: null
duplicate_of: null
---
The plan says default totals count each owned request once and that ambiguous candidates are preserved, but never says whether requests with ambiguous or unknown ownership are included in totals, excluded, or reported in a separate bucket. Done when the default totals rule covers owned, ambiguous and unknown-ownership requests and how they surface in reports and JSON.
