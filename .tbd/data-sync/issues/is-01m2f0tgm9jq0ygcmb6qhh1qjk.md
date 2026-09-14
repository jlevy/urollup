---
type: is
id: is-01m2f0tgm9jq0ygcmb6qhh1qjk
title: Build benchmark generator, harness and CI jobs
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tk5g8r3bxm5s2r4mfajg
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:24.265Z
updated_at: 2026-09-14T19:58:21.449Z
---
Seeded corpus generator (bench-small, bench-1g), harness, bench-pr and scheduled bench-1g jobs, first reference-laptop results, and summary size measurement with the request index on the representative corpus.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Record macOS peak phys_footprint beside getrusage peak RSS (RSS understates macOS memory); report cold and warm runs as distributions; add a scale test that catches quadratic reconciliation.
- Calibrate bench corpora against measured distributions: local spike (Claude p50 0.70 MB, p99 24 MB, max 49 MB; Codex p50 0.38 MB, p99 36 MB, max 388 MB, lines up to 16 MB) and ccusage's generator profile; include block-split Claude requests, repeated Codex snapshots and a pre-0.84.0 Pi message_update log.
- Spike baseline on M1 Pro under load: prefiltered typed parse ~3 s for 30 days and ~7 s for all history on 10 threads (explorations/log-throughput).
