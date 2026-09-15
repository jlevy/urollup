---
type: is
id: is-01m2khsqrw2qc4khnkxmca2p88
title: Extend ccusage reconciliation harness to costs
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
  - parity
dependencies:
  - type: blocks
    target: is-01m2f0th11tcymcgy8zqfqkn2g
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-15T22:08:02.330Z
updated_at: 2026-09-15T22:08:49.071Z
---
Milestone 0.4: extend the ccusage reconciliation harness to costs, per the plan's "ccusage reconciliation harness" section and design §4.5.

- Run the 0.1 command paths with --mode calculate (never auto or display, which mix recorded costUSD) plus --breakdown for per-model cost rows.
- Shared rates: a generator writes a urollup:PriceTable/v1 override for the fixture models from the LiteLLM snapshot ccusage 20.0.20 embeds (commit 1a183ef), passed with --prices, so rate sources cannot differ; ccusage f64 amounts must match urollup exact decimals within USD 0.000001 per row.
- A second run with urollup's bundled table reports rate-source differences for information and never fails.
- Pricing ledger entries from the research brief: fuzzy model matching, marginal per-category LiteLLM long-context rates versus urollup's whole-request band, unpriced tokens shown as cost 0 versus unpriced coverage, and Codex tiers taken from config.toml.
