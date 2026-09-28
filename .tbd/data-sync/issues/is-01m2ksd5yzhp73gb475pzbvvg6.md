---
type: is
id: is-01m2ksd5yzhp73gb475pzbvvg6
title: "Acceptance G1: roll up this project's own Claude Code sessions end to end"
kind: task
status: open
priority: 1
version: 8
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
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:20:59.482Z
updated_at: 2026-09-28T04:00:33.771Z
---
Run urollup against the maintainer's real local logs for this repository: report --current inside a session, sessions and daily over every Claude Code session of this project including its subagent sessions. Verify subagent usage attributes to the parent under --scope descendants, nothing is double counted, coverage gaps are explicit, and exit codes are correct. Reconcile token totals per session and per day against pinned ccusage, explaining every difference. Record aggregates only; never commit log content, paths or IDs.

## Notes

2026-09-19: Moved dependency from Phase 1 to Phase 2.

2026-09-27: Maintainer approved practical memory/scale acceptance in place of the representative 512 MiB and 10-second gates. G1 still waits on uro-zrr0, which now owns process-wide admission, density-scale proofs and representative evidence. This policy change does not accept G1 accounting or parity. Private values remain local unless their publication is separately authorized.
