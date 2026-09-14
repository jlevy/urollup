---
type: is
id: is-01m2ewdgre68gzmjq82v8kevhv
title: "Plan: current-session detection, session selection and hierarchy crawl"
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
created_at: 2026-09-14T02:37:24.108Z
updated_at: 2026-09-14T03:12:31.399Z
started_at: 2026-09-14T02:38:24.300Z
closed_at: 2026-09-14T03:12:31.396Z
close_reason: "Added Workflows and session selection: current-session detection order (--hook-input, agent env vars, else exit 2; --latest guarded), selection flags, disk-wide inventory, tree command and hierarchy crawl; CLI examples and Phase 1 updated."
resolution: null
duplicate_of: null
---
Carry the common workflows into the plan: a current-session command with auto-detection and explicit override for Claude and Codex (with a clear error when detection is ambiguous), selection flags (date range, session IDs, platform, project, all), disk-wide discovery, and a crawler that builds the parent-to-subagent session tree used by --scope descendants. Update CLI examples, Sources section and Phase 1 checklist.
