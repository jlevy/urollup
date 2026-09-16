---
type: is
id: is-01m2keggm34ksgr8bfxr9sr7jz
title: Add remaining CI jobs and AGENTS.md routes
kind: task
status: open
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.5
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-15T21:10:34.343Z
updated_at: 2026-09-16T03:05:22.334Z
---
Milestone 0.5: CI jobs beyond the minimal 0.1 gates (cross-target lint, semver, attestations dry run, contract gate, benchmarks) and AGENTS.md routes per design §8.2 and the Rust engineering baseline.

## Notes

Platform-gated code now exists: crates/urollup-core/src/sources/roots.rs and its tests gate symlink handling on cfg(unix), and the first Windows-only break was an unused import behind that gate, caught only by the windows-2025 CI job after a push (fixed in 109bd73). The cross-target lint job is the local guard for that class of break, so it is worth pulling forward rather than landing with the rest of this bead.
