---
type: is
id: is-01m2f0th11tcymcgy8zqfqkn2g
title: Establish feature matrix against ccusage and agentfdr
kind: task
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
  - parity
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-14T03:54:24.672Z
updated_at: 2026-09-15T22:08:46.221Z
---
Measured feature matrix against pinned ccusage and agentfdr, explaining disagreements from source records rather than treating either tool as an oracle. Milestone 0.5 extends the ccusage reconciliation harness (uro-jumy tokens, uro-qp7j local diff, uro-ne7g costs) to every shared use case in design §10.6 and records measured status in that table; see the plan's "ccusage reconciliation harness" section and the research brief's "ccusage Feature Inventory".

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Name the exact ccusage command path per row (daily vs monthly, with or without --since, JSON vs table for Codex), pin --offline and --mode calculate, and list expected disagreements: week start, session windows, Codex session identity by path, Pi forks, advisor and progress records, pricing differences.
- Add squares' readers as a labeled third comparator for token and time figures.

2026-09-15 ccusage parity review (design §10.6, 42 use cases: 17 covered, 16 covered better, 6 partial, 1 gap, 2 intentionally unsupported; ccusage 20.0.20 at bd7f89b is still the latest release):
- 0.5 cases: weekly and monthly with explicit week starts on both sides (claude weekly defaults to Sunday, unified to Monday); claude daily --instances and --project against --group-by project and --project; DST and interval-boundary fixtures; unified reports; every other §10.6 row with a ccusage counterpart.
- Add request-attributed explained amounts to the local diff once the requests command exists: each local ledger entry names the urollup diagnostic or request property identifying affected requests.
- Replace §10.6 design status with measured status per row; keep blocks out of comparison (Decision 10) and --mode auto/display out of cost comparison.
- Re-evaluate the ccusage pin against the latest release past the 14-day cool-off; unreleased main fixes (a4b8420, 15b3bef, 809eeb6, b2809fa, 527ec3a) should retire ledger entries when released.
- Candidate follow-ups tracked separately: uro-vccm and uro-iui3 (statusline), uro-00fc (report presentation), uro-8ypz (MCP), uro-uyq7 (other agents).
