---
type: is
id: is-01m2f0thbqw3mtphc4a5k1pth4
title: "Add serve: read-only HTTP API and embedded web UI"
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:25.014Z
updated_at: 2026-09-14T23:57:37.225Z
---
Read-only HTTP API and embedded UI over the same snapshots and query engine, with the planned security controls (loopback, per-launch token, Host/Origin checks, no CORS) and coverage badges.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Cap evidence payloads at the HTTP boundary with a byte count and explicit truncation flag, format evidence lazily, and render only server-computed ratios.

- Serving separability (confirmed 2026-09-14): implement `serve` as a self-contained module behind the `serve` feature, calling only urollup-core's public query API; no other CLI module imports from it; web bundle under crates/urollup/assets/web/ included only with the feature, so it can later move to its own crate.
