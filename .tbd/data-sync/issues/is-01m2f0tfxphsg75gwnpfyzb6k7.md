---
type: is
id: is-01m2f0tfxphsg75gwnpfyzb6k7
title: Add reviewed price table and repricing
kind: task
status: open
priority: 2
version: 9
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
dependencies:
  - type: blocks
    target: is-01m2khsqrw2qc4khnkxmca2p88
  - type: blocks
    target: is-01m2kswxt5gxe3s49wch9dss46
  - type: blocks
    target: is-01m2kt2ce2nta6ssw3qr0rw2zk
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-14T03:54:23.542Z
updated_at: 2026-09-16T00:32:34.237Z
---
Built-in reviewed, versioned price table, --prices overrides, staleness diagnostics and golden repricing tests. See design doc (docs/urollup-design.md): Price Table.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Explicit aliases for dated model IDs and context-variant suffixes; match speed for fast mode and cache-write duration for 1-hour writes (1h rate required as table data).
- Context bands are whole-request switches on inclusive input, highest matching threshold wins.
- A missing rate for a token category leaves those tokens unpriced; placeholder models such as codex-auto-review stay unpriced.
- Billing channel from recorded provider fields (such as Pi provider); service tier only from recorded fields, never agent configuration.
- Repricing scenarios: a promotion whose end date moves, a mid-life cache-read cut, a 200K-token band; test that the table review date is not older than any row.
