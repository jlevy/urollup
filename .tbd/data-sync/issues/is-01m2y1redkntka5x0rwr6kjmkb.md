---
type: is
id: is-01m2y1redkntka5x0rwr6kjmkb
title: Add synthetic Cursor fixtures and goldens
kind: task
status: open
priority: 2
version: 11
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: cursor@spud10
labels:
  - post-0.1
dependencies:
  - type: blocks
    target: is-01m2y1rrq5qkqz1vd10tb4qcjr
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-09-19T23:59:21.522Z
updated_at: 2026-10-11T06:29:18.420Z
started_at: 2026-09-20T09:51:23.040Z
---
Synthetic fixtures under crates/urollup-core/tests/fixtures/<dialect>/, goldens under tests/golden/e2e/<dialect>/, and result checks. Cover model/provider grouping, copies, resumes, subagents, Best-of-N, and dual-store ownership.

Never commit real Cursor logs, state databases, prompts, paths, IDs, or values. Use the brief or a structure-only sanitizer. No ccusage cursor case (ccusage 20.0.20 does not read Cursor).

## Notes

facets now locks background purpose and kimi/moonshot beside agent/chat/plan/multitask. maxMode stays undocumented as historical usage: live token bubbles have none. Still uncommitted.
