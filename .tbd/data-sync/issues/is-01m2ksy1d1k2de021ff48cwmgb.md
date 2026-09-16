---
type: is
id: is-01m2ksy1d1k2de021ff48cwmgb
title: Measure summary size with the request index on the representative corpus
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:30:11.870Z
updated_at: 2026-09-16T00:30:11.870Z
---
Milestone 0.5: check Decision 17 (request index on by default) against measured data. Design §5.2, §8.5 and Decision 17.

Acceptance:
- Measured bytes per request added by the request index on the consented representative corpus, compared with the design's estimate of roughly 40 bytes per request.
- Measured summary and bundle sizes with and without the index, and with and without the records table.
- Results recorded beside the benchmark records; if the measurement disagrees with the design's scaling note, report it for a maintainer decision rather than editing the design here.
