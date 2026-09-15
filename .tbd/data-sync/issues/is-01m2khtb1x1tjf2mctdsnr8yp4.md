---
type: is
id: is-01m2khtb1x1tjf2mctdsnr8yp4
title: Add statusline today segment after capture cache reads (Candidate)
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
  - candidate
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-15T22:08:22.077Z
updated_at: 2026-09-15T22:08:22.077Z
---
Phase 2, Candidate (design §9.1 "Statusline Command"; implement only once confirmed): add the statusline today segment, the list-price estimate of today's usage in the report timezone, once capture cache reads keep statusline within its proposed latency gate (p95 under 250 ms). Design §6.8 and §10.2. Also add a statusline session-cost case to the ccusage reconciliation harness if confirmed.
