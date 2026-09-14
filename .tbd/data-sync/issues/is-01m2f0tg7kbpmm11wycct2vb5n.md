---
type: is
id: is-01m2f0tg7kbpmm11wycct2vb5n
title: Add Phase 1 report commands
kind: task
status: open
priority: 2
version: 9
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tgm9jq0ygcmb6qhh1qjk
  - type: blocks
    target: is-01m2f0th11tcymcgy8zqfqkn2g
  - type: blocks
    target: is-01m2f0thbqw3mtphc4a5k1pth4
  - type: blocks
    target: is-01m2f0thng891bmbgwtjm3e1hy
  - type: blocks
    target: is-01m2f0tjvpecgmv6v5n6vnnrkb
  - type: blocks
    target: is-01m2gazeqdqvjq1s6xzp8k0jq1
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:23.858Z
updated_at: 2026-09-14T19:58:18.994Z
---
sources, sessions, daily, weekly, monthly, report, requests, tools and export with project, account, model, effort and observed purpose grouping, request sizes, deterministic output in every format, query files, --strict and --require-priced.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Port metaproc's cross-agent tool taxonomy with native names kept; name call-to-result time a tool interval, never latency or execution time.
- Report the pooled cache-read share from token sums; port ccusage terminal width and table rendering (MIT notice).
- Resolve relative time expressions to absolute instants in the normalized QuerySpec; check verdicts never depend on the wall clock.
- Golden test that Markdown reports are unchanged by the pinned flowmark.
- sources diagnostic for sessions near Claude Code's transcript cleanup.
- Pending decisions (branch and agent_path grouping, extended time measures, command classifier) may add work here.
