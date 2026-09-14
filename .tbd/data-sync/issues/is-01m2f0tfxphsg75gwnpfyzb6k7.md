---
type: is
id: is-01m2f0tfxphsg75gwnpfyzb6k7
title: Add reviewed price table and repricing
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tg7kbpmm11wycct2vb5n
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:23.542Z
updated_at: 2026-09-14T19:58:22.631Z
---
Built-in reviewed, versioned price table, --prices overrides, staleness diagnostics and golden repricing tests. See architecture doc: Price Table.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Explicit aliases for dated model IDs and context-variant suffixes; match speed for fast mode and cache-write duration for 1-hour writes (1h rate required as table data).
- Context bands are whole-request switches on inclusive input, highest matching threshold wins.
- A missing rate for a token category leaves those tokens unpriced; placeholder models such as codex-auto-review stay unpriced.
- Billing channel from recorded provider fields (such as Pi provider); service tier only from recorded fields, never agent configuration.
- Repricing scenarios: a promotion whose end date moves, a mid-life cache-read cut, a 200K-token band; test that the table review date is not older than any row.
