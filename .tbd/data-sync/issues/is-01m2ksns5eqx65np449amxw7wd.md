---
type: is
id: is-01m2ksns5eqx65np449amxw7wd
title: Settle the fixture cases' open reconciliation questions
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - fixtures
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:41.285Z
updated_at: 2026-09-16T00:25:41.285Z
---
Three expectations in crates/urollup-core/tests/fixtures/ are interpretations, flagged in each case's expected.json notes, and the ledger and adapters must confirm or change them (and then the fixtures).

- claude-project/cache-creation-breakdown: design 4.1 keeps both native cache-write values with a diagnostic when the cache_creation breakdown disagrees with cache_creation_input_tokens, but does not say which feeds the cache_write total. The fixture uses the flat count and reports the breakdown separately.
- claude-project/progress-nested-subagent: a progress record nesting a subagent message whose transcript was never discovered is not counted, following design 3.3 (a copy nested inside another record never counts), with a claude-nested-copy-without-original diagnostic. ccusage daily counts it. Decide between not counted, counted as the subagent's request, or an unobserved coverage gap.
- claude-project/quota-limits: a <synthetic> record with isApiErrorMessage is a synthetic event rather than a provider request (design 4.1 Calls), and its 'Claude AI usage limit reached|<epoch>' text is a provider limit observation with basis api-error-text, which design 3.1 does not list for claude-project.

Each answer either confirms the fixture or changes expected.json in the same commit.
