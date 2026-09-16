---
type: is
id: is-01m2f0thng891bmbgwtjm3e1hy
title: Add reporting skill, compare and check
kind: task
status: open
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2f0tj0cf0v7wbz9vbpywjqs
parent_id: is-01m2ksy5yrxn48thxc311sqsv6
created_at: 2026-09-14T03:54:25.327Z
updated_at: 2026-09-16T00:31:17.077Z
---
CLI-backed reporting skill plus compare and check commands.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Reporting skill model (Anthropic session-report and receipts): the CLI emits deterministic JSON, a fixed template renders it, and the agent fills only short narrative slots; log names are data, state which columns add up, unknown is not zero, never publish by default.
- Pending decision: port agentfdr's anomaly detectors (loops with retry allowance, error streaks, token spikes, stalled calls) as labeled estimates in check or the skill; avoid its cache-thrash check, reconstructed 5-hour windows and pattern-matched pricing.
