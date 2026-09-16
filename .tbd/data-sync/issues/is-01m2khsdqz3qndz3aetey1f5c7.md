---
type: is
id: is-01m2khsdqz3qndz3aetey1f5c7
title: Add ccusage reconciliation harness for token totals on fixtures
kind: task
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - parity
dependencies:
  - type: blocks
    target: is-01m2khsqbsy48eyvw5sevz9gqk
  - type: blocks
    target: is-01m2khsqrw2qc4khnkxmca2p88
  - type: blocks
    target: is-01m2f0th11tcymcgy8zqfqkn2g
  - type: blocks
    target: is-01m2ksqda8y7hfm5pstky74ms1
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-15T22:07:52.062Z
updated_at: 2026-09-16T00:26:34.689Z
---
Milestone 0.1: side-by-side ccusage reconciliation harness for token totals, per the plan's "ccusage reconciliation harness" section (Testing Strategy), the design's §10.6 ccusage use-case coverage, and the research brief's "ccusage Feature Inventory".

- tests/parity/ccusage/: separate npm project pinning ccusage 20.0.20 exactly (published 2026-08-15, past the 14-day cool-off) with a committed lockfile; install with npm ci --ignore-scripts and npm audit signatures; the supply-chain validator covers the lockfile; run the native platform binary directly and check --version; never npx, bunx, pnpm dlx, nix run or @latest; fallback is cargo build --locked of the tag commit.
- tests/parity/cases.toml, ledger.toml, compare.py (standard-library Python run through uv --config-file uv.toml run --frozen, TOML via tomllib) and unittest tests; make parity target and a CI job on every pull request that uploads the per-case report as an artifact.
- Isolated runs: temporary fixture copy, HOME/XDG dirs empty, CLAUDE_CONFIG_DIR and CODEX_HOME at the copy, no .ccusage/ folder; ccusage always --offline --json; same --timezone both sides (UTC and America/Los_Angeles); inclusive --until mapped to half-open bounds; NO_COLOR=1, LOG_LEVEL=0.
- Cases: ccusage claude and codex daily and session, with and without --since, plus unified daily, on the claude-project and codex-rollout fixtures, against urollup daily and sessions with the matching --scope.
- Compare uncached input, output, cache writes, cache reads, total (and Codex reasoning) per day and per session exactly; one-sided rows are differences.
- Ledger: every difference must match exactly one cited entry (cause, citation, design link, retirement condition); unmatched differences, stale entries, double matches and uncited entries fail. Seed with the brief's nested-null, sidechain replay (a4b8420), Codex dedupe-key, fractional-timestamp, .jsonl.zst/token_usage_record/subagent_history_start_ordinal, Codex cache-creation-0 (15b3bef), last-activity session filter and synthetic/advisor model entries.
