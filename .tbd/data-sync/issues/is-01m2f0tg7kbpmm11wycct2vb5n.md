---
type: is
id: is-01m2f0tg7kbpmm11wycct2vb5n
title: Add report, daily and sessions in table and JSON
kind: task
status: open
priority: 2
version: 19
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
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
  - type: blocks
    target: is-01m2keg1mz8y0a8jhxrgta266x
  - type: blocks
    target: is-01m2khsdqz3qndz3aetey1f5c7
  - type: blocks
    target: is-01m2khsrbqk941a7egfkbzc70p
  - type: blocks
    target: is-01m2khsrseppmwbrx7ykx4c1dp
  - type: blocks
    target: is-01m2ks6m5jg3tqh3dv22mqmv17
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-14T03:54:23.858Z
updated_at: 2026-09-16T00:20:59.482Z
---
Milestone 0.1: report, daily and sessions commands in table and JSON formats with project, account, model and effort grouping, request sizes, deterministic output, exit codes and CLI goldens, per design §6.3-§6.5, §4.1 and §4.3; list-price estimates wait for milestone 0.4. Remaining commands and formats are milestone 0.5.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Port metaproc's cross-agent tool taxonomy with native names kept; name call-to-result time a tool interval, never latency or execution time.
- Report the pooled cache-read share from token sums; port ccusage terminal width and table rendering (MIT notice).
- Resolve relative time expressions to absolute instants in the normalized QuerySpec; check verdicts never depend on the wall clock.
- Golden test that Markdown reports are unchanged by the pinned flowmark.
- sources diagnostic for sessions near Claude Code's transcript cleanup.
- Pending decisions (branch and agent_path grouping, extended time measures, command classifier) may add work here.

Must include end-to-end goldens and result checks on the fixture cases (uro-3ht1, uro-xsfj): one transcript golden per case holding report, daily and sessions in table and JSON, plus `make e2e-results` comparing reconciled totals with expected.json. Landing report JSON also means replacing the provisional extractors in scripts/check-e2e-results.mjs, updating tests/golden/samples/outputs/, and deleting the pending entries in tests/golden/e2e.config.json, which fail the run once the commands exist.
