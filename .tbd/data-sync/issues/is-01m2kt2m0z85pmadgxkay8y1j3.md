---
type: is
id: is-01m2kt2m0z85pmadgxkay8y1j3
title: Add JSONL, CSV and Markdown output formats
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:32:42.013Z
updated_at: 2026-09-16T03:02:02.846Z
closed_at: 2026-09-16T03:02:02.845Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-81ku is the original.
resolution: duplicate
duplicate_of: is-01m2ksxmcb91f3jse6mz6207ck
---
Milestone 0.5: the remaining output renderings over the shared report data. Design §6.4, §6.6 and Decision 15.

Acceptance:
- --format jsonl, csv and markdown all derive from the same report data as the terminal table and JSON; JSONL and CSV values are never formatted for display.
- A JSONL export ends with a completion record; a JSON document is written only after the query completes; cancellation writes neither.
- stdout carries only the requested format, and diagnostics, progress and logs go to stderr.
- Markdown matches the design's session report and weekly rollup examples (§6.6), including the tokens, cost, sizes, tools, coverage and unresolved lines.
- Goldens for each format over the same fixture selection, byte-identical across runs.
