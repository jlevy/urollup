---
type: is
id: is-01m2ksmr7x9e0t324y8zfx071j
title: Confirm the cache-write category for writes with no recorded lifetime
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - design-decision
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:07.579Z
updated_at: 2026-09-16T00:25:07.579Z
---
Design 4.1 names 5-minute and 1-hour cache writes as separate measures, but several records report a cache write without its lifetime: a Codex cache_write_tokens value, and a Claude cache_creation_input_tokens without its cache_creation breakdown. uro-spce added a third disjoint category, cache_write_unspecified, so the categories still sum and an unknown lifetime is never silently counted as 5-minute (pricing a 1-hour write as 5-minute would understate cost).

Confirm the category and its name before the summary contract fixes field names in milestone 0.2, or decide instead that adapters infer the default 5-minute lifetime with a diagnostic. See crates/urollup-core/src/ledger/tokens.rs (TokenMeasures) and design 4.1.
