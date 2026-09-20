---
type: is
id: is-01m2y4ydxjz5xpq81vf3ecd9rv
title: Enforce observation capacity during decoding before retaining over-budget rows
kind: bug
status: in_progress
priority: 1
version: 7
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
updated_at: 2026-09-20T04:57:41.138Z
started_at: 2026-09-20T01:36:18.143Z
---
Audit R1, PR #10 and #12. Both adapters finish try_read_in_parallel and collect all decoded rows before reconcile_with_capacity checks input.requests.len(). A 32 MiB synthetic corpus with --max-rows 1 retained 19,757 Codex observations and reached 35.44 MiB physical footprint before exit 1. The check cannot protect decode from OOM. Reserve a shared per-agent row budget before retaining observations (including pending Codex and Claude representations), propagate cancellation/errors, and document row budget versus whole-process memory. Cover rejection before reading the rest of a large synthetic stream, deterministic failure and successful accounting parity.

## Notes

Integrated through PR12: configured ObservationCapacity is cloned once into shared admission, preserving maximum and label. Independent review found no blocker. Regression commit667a06a now passes all18 early-refusal cases and9 exact-capacity success cases across Claude/Codex direct/Codex pending with1/2/8 workers; CLI exact labels, empty stdout and stricter-limit rules pass (11 process tests). Integrated full make check and fresh stack CI running; final published heads may change for standalone PR10 test API correction.
