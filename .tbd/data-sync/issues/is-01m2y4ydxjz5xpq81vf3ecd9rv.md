---
type: is
id: is-01m2y4ydxjz5xpq81vf3ecd9rv
title: Enforce observation capacity during decoding before retaining over-budget rows
kind: bug
status: closed
priority: 1
version: 8
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-20T00:55:03.344Z
updated_at: 2026-09-20T05:17:03.935Z
started_at: 2026-09-20T01:36:18.143Z
closed_at: 2026-09-20T05:17:03.934Z
close_reason: Fixed in PR10 at b390982f0efb7323f46ebe34e471be15e4f3bfeb; all 15 final hosted checks passed. Early-admission regressions, measured synthetic refusal and actual Ubuntu/macOS scale execution are recorded in the governing review. Full integrated make check passed, including all 30 negative gate probes.
resolution: null
duplicate_of: null
---
Audit R1, PR #10 and #12. Both adapters finish try_read_in_parallel and collect all decoded rows before reconcile_with_capacity checks input.requests.len(). A 32 MiB synthetic corpus with --max-rows 1 retained 19,757 Codex observations and reached 35.44 MiB physical footprint before exit 1. The check cannot protect decode from OOM. Reserve a shared per-agent row budget before retaining observations (including pending Codex and Claude representations), propagate cancellation/errors, and document row budget versus whole-process memory. Cover rejection before reading the rest of a large synthetic stream, deterministic failure and successful accounting parity.

## Notes

Integrated through PR12: configured ObservationCapacity is cloned once into shared admission, preserving maximum and label. Independent review found no blocker. Regression commit667a06a now passes all18 early-refusal cases and9 exact-capacity success cases across Claude/Codex direct/Codex pending with1/2/8 workers; CLI exact labels, empty stdout and stricter-limit rules pass (11 process tests). Integrated full make check and fresh stack CI running; final published heads may change for standalone PR10 test API correction.
