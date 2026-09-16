---
type: is
id: is-01m2ksxmcb91f3jse6mz6207ck
title: Add JSONL, CSV and Markdown output formats
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies:
  - type: blocks
    target: is-01m2ksd8fxt209yqwabvy5r8kd
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:29:58.530Z
updated_at: 2026-09-16T00:31:54.529Z
---
Milestone 0.5: the remaining output renderings over the shared report data. Design §6.4, §6.6 and Decision 15.

Acceptance:
- --format jsonl, csv and markdown all derive from the same report data as the terminal table and JSON; JSONL and CSV values are never formatted for display.
- A JSONL export ends with a completion record; a JSON document is written only after the query completes; cancellation writes neither.
- stdout carries only the requested format, and diagnostics, progress and logs go to stderr.
- Markdown matches the design's session report and weekly rollup examples (§6.6), including the tokens, cost, sizes, tools, coverage and unresolved lines.
- Goldens for each format over the same fixture selection, byte-identical across runs.
