---
type: is
id: is-01m2y4ydxjz5xpq81vf3ecd9rv
title: Enforce observation capacity during decoding before retaining over-budget rows
kind: bug
status: in_progress
priority: 1
version: 6
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
updated_at: 2026-09-20T04:44:24.846Z
started_at: 2026-09-20T01:36:18.143Z
---
Audit R1, PR #10 and #12. Both adapters finish try_read_in_parallel and collect all decoded rows before reconcile_with_capacity checks input.requests.len(). A 32 MiB synthetic corpus with --max-rows 1 retained 19,757 Codex observations and reached 35.44 MiB physical footprint before exit 1. The check cannot protect decode from OOM. Reserve a shared per-agent row budget before retaining observations (including pending Codex and Claude representations), propagate cancellation/errors, and document row budget versus whole-process memory. Cover rejection before reading the rest of a large synthetic stream, deterministic failure and successful accounting parity.

## Notes

R1 implemented on owning PR10 as d16542e before restack; full make check passed including all 30 negative gate proofs. Shared per-adapter admission reserves before retaining request-bearing decoded rows and cancels readers; metadata-only Codex records do not consume slots. Restack is propagating this through PR11 typed decoders and PR12 configurable capacity; integrated early-refusal and exact-label regressions plus final CI remain.
