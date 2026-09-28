---
type: is
id: is-01m2f0thng891bmbgwtjm3e1hy
title: Add reporting skill, compare and check
kind: task
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2f0tj0cf0v7wbz9vbpywjqs
parent_id: is-01m2ksy5yrxn48thxc311sqsv6
created_at: 2026-09-14T03:54:25.327Z
updated_at: 2026-09-28T02:18:23.878Z
---
CLI-backed reporting skill plus compare and check commands. The maintainer explicitly wants an easy-to-use skill that answers usage questions across supported dimensions, invokes deterministic CLI queries, explains results through an agent, and can feed a pivot table or browsable hierarchy from the same outputs. Build on uro-x8r8 reusable snapshots and uro-qvp1 joint grouping; ingest once per snapshot and never reconstruct joint cells from independent marginals. Expose available dimensions and metric coverage, preserve unknowns and non-additive metrics, and keep local logs and derived artifacts private unless sharing is explicitly approved. Use a common versioned query/result contract so text, pivot and hierarchy presentations agree. A lightweight token-reporting skill is a proposed early slice and need not depend on the full web server; do not silently move the accepted milestone or claim unavailable pricing/tool/time capabilities. Preserve the existing cloud-export, compatible pinned-binary, compare/check and deterministic-rendering requirements.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Reporting skill model (Anthropic session-report and receipts): the CLI emits deterministic JSON, a fixed template renders it, and the agent fills only short narrative slots; log names are data, state which columns add up, unknown is not zero, never publish by default.
- Pending decision: port agentfdr's anomaly detectors (loops with retry allowance, error streaks, token spikes, stalled calls) as labeled estimates in check or the skill; avoid its cache-thrash check, reconstructed 5-hour windows and pattern-matched pricing.
