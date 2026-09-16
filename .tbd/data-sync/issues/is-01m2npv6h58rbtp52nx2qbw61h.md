---
type: is
id: is-01m2npv6h58rbtp52nx2qbw61h
title: Finalize alpha 0.1 readiness and packaging plan against Rust CLI guidance
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
delegate: claude-code@spud10.local
labels:
  - release
  - planning
dependencies: []
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
hold: null
hold_until: null
created_at: 2026-09-16T18:14:41.956Z
updated_at: 2026-09-16T18:37:28.959Z
started_at: 2026-09-16T18:14:52.332Z
closed_at: 2026-09-16T18:37:28.957Z
close_reason: Finalized and reconciled the 0.1.0 readiness and publishing plan with tbd PR 302; corrected release tracking, updated PR 4, and passed local and remote gates.
resolution: null
duplicate_of: null
---
Reconcile milestone 0.1 readiness, the first-release publishing spec, and release beads against the updated Rust CLI, release engineering, CI/gates, agent acquisition, Python CLI boundary, and supply-chain guidance in jlevy/tbd PR #302. Make the alpha test gate explicit, align Cargo 1.90+ workspace publication, Maturin bin wheel validation, exact uv commands, scoped registry bootstrap, build-once artifact/evidence handling, CI and recovery, repair dependencies and future-binding ownership, and leave one ordered critical path to an end-to-end testable 0.1.0 candidate.

## Notes

Reconciled the active publishing spec, main plan and design with tbd PR 302. Defined four readiness gates, fixed the first public alpha at 0.1.0 after milestone 0.1, selected macOS 11.0, adopted Cargo 1.90-or-newer workspace publication, tightened wheel and bootstrap-token validation, and corrected bead ownership and dependencies.
