---
type: is
id: is-01m2ewa21mysjz5ryzf1k9my1h
title: "Research: add a synthetic worked double-counting example"
kind: task
status: closed
priority: 2
version: 4
spec_path: docs/project/research/research-2026-09-13-portable-agent-usage.md
delegate: claude-code@spud10.local
labels:
  - research
dependencies:
  - type: blocks
    target: is-01m2ewe3whtebs3z0pgza28pj4
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:30.739Z
updated_at: 2026-09-14T02:55:07.182Z
started_at: 2026-09-14T02:38:24.602Z
closed_at: 2026-09-14T02:55:07.181Z
close_reason: Added synthetic double-counting example (Claude repeated content blocks plus resume replay; Codex cumulative counters) with naive vs reconciled totals, verified against session-report, agentfdr, Metabrowser and ccusage v20.0.20 sources.
resolution: null
duplicate_of: null
---
The brief excludes private examples, leaving its core claims without a concrete illustration. Done when a small synthetic example shows how naive summing overcounts (e.g. one response whose usage repeats across several content blocks, or a forked history re-read) and what the reconciled count should be.
