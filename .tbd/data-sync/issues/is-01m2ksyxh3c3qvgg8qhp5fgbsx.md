---
type: is
id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
title: Release, distribution and pre-1.0 validation
kind: epic
status: in_progress
priority: 1
version: 12
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
  - is-01m2ksz8r23b3kmymq8g06c90a
  - is-01m2nhepspk2hs3g02ymfdv9sa
  - is-01m2npv6h58rbtp52nx2qbw61h
hold: null
hold_until: null
created_at: 2026-09-16T00:30:40.663Z
updated_at: 2026-09-16T18:14:41.956Z
started_at: 2026-09-16T16:47:20.853Z
---
The plan's Rollout Plan: build and publish urollup as a standalone product, and validate it before 1.0. Covers the platform matrix, release channels, verification and versioning, release documentation, the pre-1.0 release checklist with shadow-mode validation, and the consented-corpus ground-truth check.

Plan: Rollout Plan and Testing Strategy (Ground truth). Design: Decision 27 (release scope) and §5.6 (contract versioning). Baseline: the Rust CLI engineering baseline's targets, channels and versioning.

## Notes

Focused implementation plan created at docs/project/specs/active/plan-2026-09-16-first-release-publishing.md. It selects GitHub Releases, crates.io and a PyPI urollup Maturin binary wheel, defines the native archive and wheel matrices, and gives the rehearsal, first-publish, verification and recovery sequence. Importable Python bindings remain separately tracked by uro-8vvf.
