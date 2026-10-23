---
type: is
id: is-01m5nfxwhw20wwfxgqy8cfe5cd
title: Quantify usage excluded as coverage gaps
kind: feature
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - coverage
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:55.483Z
updated_at: 2026-10-23T23:00:55.483Z
---
Whole-history runs on real Codex history emit codex-history-boundary-unverified thousands of times and mark the report partial, but no output says how many requests or tokens were excluded, so a user cannot tell a negligible gap from a large one. Report, per gap kind, the excluded request count and excluded token measures (and the number of threads affected) wherever coverage appears, without adding them to totals. Builds on the coverage block bead in this phase. Related: uro-akgu (diagnostic names no session).
