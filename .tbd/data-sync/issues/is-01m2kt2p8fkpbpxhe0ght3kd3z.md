---
type: is
id: is-01m2kt2p8fkpbpxhe0ght3kd3z
title: Build the benchmark harness and record reference-laptop results
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies:
  - type: blocks
    target: is-01m2kt2rajxve97nh4pfasn4n6
  - type: blocks
    target: is-01m2kt2sa9a5gpv2n3x2t3zva8
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:32:44.302Z
updated_at: 2026-09-16T03:01:55.475Z
closed_at: 2026-09-16T03:01:55.474Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-2kd6 is the original.
resolution: duplicate
duplicate_of: is-01m2ksxtta6tgmz0msmkbk6r3f
---
Milestone 0.5: measure the plan's performance targets. Design §8.3 and §8.5, Decision 26, and the plan's performance targets.

Acceptance:
- Release binaries run as subprocesses: 3 warmup and 10 measured runs per command on a warm filesystem cache (cold runs recorded, not gated), with peak RSS from getrusage; resident queries use 5 warmup and 50 measured requests.
- Each run writes a JSON record of build, machine, corpus, command, run counts, cache state, timings, peak RSS and throughput; reference-laptop records are committed under bench/results/ for each release.
- Gated commands write no capture entries; capture writes are recorded separately (uro-72xi).
- First reference-laptop results recorded on an Apple silicon laptop with at least 10 cores, 16 GiB RAM and an internal SSD, on AC power and otherwise idle: bench-small `daily` and `report --session` (proposed gate, median under 1 s) and bench-1g `daily`, `monthly --group-by account,model,effort` and `sessions` (median under 10 s, peak RSS under 512 MiB).
- Targets are proposed gates, not measured claims, and change in the plan only with recorded results.

## Notes

From the source reviews: record macOS peak phys_footprint beside getrusage peak RSS, since RSS understates macOS memory; report cold and warm runs as distributions; add a scale test that catches quadratic reconciliation. Spike baseline on an M1 Pro under load: prefiltered typed parse about 3 s for 30 days and 7 s for all history on 10 threads (explorations/log-throughput).
