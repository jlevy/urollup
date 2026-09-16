---
type: is
id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
title: First release 0.1.0 packaging and validation
kind: epic
status: in_progress
priority: 1
version: 14
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
delegate: claude-code@spud10.local
labels:
  - release
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2ksyz8vzvrj9r0tj246h08q
  - is-01m2ksz138y5h2rwsesrv1eve7
  - is-01m2ksz2s56jbdehka0ddtx51j
  - is-01m2ksz57qjv1wkzygfgynae26
  - is-01m2ksz7d707rz38cfe8gh2xbj
  - is-01m2npv6h58rbtp52nx2qbw61h
hold: null
hold_until: null
created_at: 2026-09-16T00:30:40.663Z
updated_at: 2026-09-16T18:37:55.960Z
started_at: 2026-09-16T16:47:20.853Z
---
Build, rehearse, publish and verify the first public urollup alpha after milestone 0.1 acceptance. This epic owns GitHub archives, crates.io workspace publication, PyPI Maturin binary wheels, release documentation, protected publishing, post-publish probes and recovery. Later Phase 1 features, provider-export validation before 1.0 and future Python bindings are tracked outside this first-release epic.

## Notes

The focused publishing plan is active and implementation-ready. It follows the release-engineering and Rust-release split in tbd PR 302: Cargo 1.90-or-newer workspace publication, macOS 11.0, exact wheel script and RECORD validation, exact-version uv paths, narrowly scoped crates.io bootstrap credentials, separate artifact manifest and release evidence, and independent channel recovery.
