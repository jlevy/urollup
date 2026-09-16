---
type: is
id: is-01m2kt2ta16w8bgb4150fkktks
title: Extend the ccusage reconciliation harness to the 0.5 cases
kind: task
status: closed
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
  - parity
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:32:48.443Z
updated_at: 2026-09-16T03:01:55.717Z
closed_at: 2026-09-16T03:01:55.716Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-ey63 is the original.
resolution: duplicate
duplicate_of: is-01m2ksy3sr8ges0ra56f92ftxg
---
Milestone 0.5: grow the parity harness to the full shared feature matrix. Plan, ccusage reconciliation harness (milestone 0.5 row); design §10.6.

Acceptance:
- New cases: weekly and monthly with the week start set explicitly on both sides (ccusage claude weekly defaults to Sunday and its unified weeks to Monday), `claude daily --instances` and `--project` against --group-by project and --project, DST and interval-boundary fixtures, unified reports, and every other §10.6 row with a ccusage counterpart.
- The local diff gains request-attributed explained amounts: each local ledger entry names the urollup diagnostic or request property that identifies the affected requests, and the script sums those requests' tokens in memory while still writing only aggregates.
- blocks stays out of comparison (Decision 10), and --mode auto and display stay out of cost comparison.
- The ccusage pin is re-evaluated against the latest release past the 14-day cool-off; unreleased main fixes (a4b8420, 15b3bef, 809eeb6) retire their ledger entries once released.
- The comparator still fails when a difference matches no entry, an entry matches no difference, two entries match one difference, or an entry lacks a citation.
