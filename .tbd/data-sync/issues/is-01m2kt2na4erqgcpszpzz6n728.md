---
type: is
id: is-01m2kt2na4erqgcpszpzz6n728
title: Add saved query files and --strict coverage mode
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies:
  - type: blocks
    target: is-01m2kt2ydvnrafh90m74zhf90v
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:32:43.315Z
updated_at: 2026-09-16T03:02:03.621Z
closed_at: 2026-09-16T03:02:03.613Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-9cvc is the original.
resolution: duplicate
duplicate_of: is-01m2ksxr5pp72ezkfb768wgva3
---
Milestone 0.5: reusable queries and the strict coverage gate. Design §6.4, §6.5, and §9.1 (Strict Mode, Configuration Defaults).

Acceptance:
- Every command compiles to a versioned QuerySpec of sources, snapshot, selection, time range, timezone, filters, grouping, scope, measures, ordering, pagination and pricing policy including --prices files; `--query <file>` reruns a saved one.
- A relative --since or --until resolves to an absolute instant in the normalized QuerySpec, so every report records the interval it used.
- Saved QuerySpec files are the only report presets: no configuration file sets default flags, and the platform config directory holds only sources.yaml and prices.yaml, so a report depends only on its command line, query file, sources and price files.
- --strict exits 3 on any coverage gap, including nonzero unresolved usage; goldens cover exits 0, 2 and 3 and the stage order in §6.5.
