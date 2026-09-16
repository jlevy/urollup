---
type: is
id: is-01m2nrq2x33grg7kg9v3c9psph
title: Add terminal-aware color as a baseline CLI contract
kind: feature
status: in_progress
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - cli
  - testing
dependencies:
  - type: blocks
    target: is-01m2nw89ncs6egan0784hkk356
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-09-16T18:47:24.321Z
updated_at: 2026-09-16T19:49:30.689Z
started_at: 2026-09-16T19:49:30.687Z
---
Milestone 0.1 table stakes for a modern Rust CLI. Human-facing help, usage, diagnostics and terminal tables use color automatically only when their target stream is an interactive terminal. Redirected or piped output is stable plain text, and machine-readable formats never contain ANSI escapes. Provide an unconditional disable through the documented CLI color policy and NO_COLOR; reconcile exact behavior with fdu and the shared tbd Rust CLI guidance. Add deterministic tests for interactive auto-color, redirected auto/plain output, explicit disable, NO_COLOR, machine-format output, and help/error paths.
