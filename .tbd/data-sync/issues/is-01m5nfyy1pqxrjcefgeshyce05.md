---
type: is
id: is-01m5nfyy1pqxrjcefgeshyce05
title: Give a single subagent file named by --source its normal identity
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:29.781Z
updated_at: 2026-10-23T23:01:29.781Z
---
roots.rs:98-101 uses a lone file's parent directory as its root, so a Claude subagent transcript passed directly with --source gets a different analytical ID than under its project root and adds two empty phantom rows; a project-level agent-*.jsonl (older Claude Code layouts, unverified) or a session directory passed as --source becomes a fake main session, and --session <parent> then misses that usage (location_ids 1839-1854, cli.rs:777-812). Fix: recognize subagent files by their sidechain records and parent session, not only by a subagents path component.
