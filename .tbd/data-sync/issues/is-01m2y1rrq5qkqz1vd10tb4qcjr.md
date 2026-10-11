---
type: is
id: is-01m2y1rrq5qkqz1vd10tb4qcjr
title: Verify Cursor facts on consented local stores
kind: task
status: open
priority: 2
version: 7
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: cursor@spud10
labels:
  - post-0.1
dependencies: []
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-09-19T23:59:32.067Z
updated_at: 2026-10-11T06:29:19.073Z
started_at: 2026-09-20T09:51:23.062Z
---
Consented local verification that prints aggregates only, under the same privacy sentinels as make e2e-local. Confirm or revise locked format facts against real stores. Do not commit that corpus, paths, IDs, or values.

## Notes

2026-09-20: a local run against a real Cursor store reconciled with no unresolved requests and showed that recent store bubbles carry no token counts and that messageRequestContext is prompt context, not usage (values kept local). The code now lives in draft PR #20 (stack #21).
