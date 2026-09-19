---
type: is
id: is-01m2ksd5yzhp73gb475pzbvvg6
title: "Acceptance G1: roll up this project's own Claude Code sessions end to end"
kind: task
status: open
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - acceptance
dependencies:
  - type: blocks
    target: is-01m2ksd74mkre0zjh8rka36a4p
  - type: blocks
    target: is-01m2ksz7d707rz38cfe8gh2xbj
  - type: blocks
    target: is-01m2pkgva22qd452me3cdjc1fq
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:20:59.482Z
updated_at: 2026-09-19T07:40:44.854Z
---
Run urollup against the maintainer's real local logs for this repository: report --current inside a session, sessions and daily over every Claude Code session of this project including its subagent sessions. Verify subagent usage attributes to the parent under --scope descendants, nothing is double counted, coverage gaps are explicit, and exit codes are correct. Reconcile token totals per session and per day against pinned ccusage, explaining every difference. Record aggregates only; never commit log content, paths or IDs.

## Notes

2026-09-19: Now depends on uro-n1cp so G1 runs on the compact engine after the 512 MiB whole-history target. Blockers uro-20ck, uro-d135, uro-qp7j and uro-4gxg are already closed.
