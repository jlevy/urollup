---
type: is
id: is-01m2nrrbmesqz5w2hzh0phza1v
title: Add interactive progress indication as a baseline CLI contract
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
created_at: 2026-09-16T18:48:06.029Z
updated_at: 2026-09-16T19:49:31.051Z
started_at: 2026-09-16T19:49:31.048Z
---
Milestone 0.1 table stakes for a modern Rust CLI. Operations with noticeable latency show progress by default only for interactive human use. Progress is written to stderr so stdout remains composable, is automatically suppressed when the relevant stream is non-interactive and for machine-oriented workflows, and is unconditionally disabled by --no-progress. The indicator must clean up correctly on success and error and must not leak ANSI/control sequences into redirected output. Add deterministic tests for TTY default-on, pipe/non-TTY default-off, --no-progress, stdout separation, and success/error cleanup; reconcile implementation guidance with fdu and shared tbd Rust CLI guidance.
