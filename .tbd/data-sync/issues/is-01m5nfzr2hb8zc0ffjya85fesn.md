---
type: is
id: is-01m5nfzr2hb8zc0ffjya85fesn
title: Show list-price estimates and pricing coverage in report, daily and sessions
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - pricing
  - alpha
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2khsqrw2qc4khnkxmca2p88
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:56.432Z
updated_at: 2026-10-23T23:02:40.610Z
---
The CLI integration of supported cost rollups required by the frozen alpha scope (uro-aakb); uro-wuby owns the table, uro-neii matching, uro-7nhm staleness and --require-priced. Add a list-price estimate to report totals and breakdowns, daily rows and sessions rows in table and JSON output, with the bundled table version and review date, priced, default-assumed and unpriced request and token counts (design §6.6 examples), and unknown usage shown as unpriced, never zero. Exact decimal amounts in JSON; currency stated. Start from the money and price-table work on feat/pricing-engine. Goldens cover priced, assumed and unpriced rows.
