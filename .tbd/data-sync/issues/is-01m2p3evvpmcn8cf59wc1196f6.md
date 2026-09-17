---
type: is
id: is-01m2p3evvpmcn8cf59wc1196f6
title: Stream or spill multi-gigabyte all-log aggregation
kind: feature
status: open
priority: 0
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - performance
  - memory
  - testing
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T21:55:09.300Z
updated_at: 2026-09-16T21:55:09.300Z
---
Replace the temporary v0.1 input-size crash barrier with a measured bounded-memory architecture for default-all and --all aggregation across multi-gigabyte Claude Code and Codex corpora. Stream or spill normalized observations and reconciliation state, enforce decoded-byte and retained-heap contracts including compressed inputs and growing snapshots, add generated large-corpus regression coverage, and complete the consented full local acceptance run with privacy-safe peak-RSS evidence. This is distinct from session-family preselection, which keeps --current and exact narrow selections usable without reading unrelated logs.
