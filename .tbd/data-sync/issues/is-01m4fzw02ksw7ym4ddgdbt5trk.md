---
type: is
id: is-01m4fzw02ksw7ym4ddgdbt5trk
title: Codex history-boundary diagnostic names no session, and its advice names a field legacy rollouts lack
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T09:28:22.353Z
updated_at: 2026-10-09T09:28:22.353Z
---
Found by PR #16 follow-up review C (C4, Low; A2 raised location first): https://github.com/jlevy/urollup/pull/16#pullrequestreview-5468251421. The codex-history-boundary-unverified detail (UNVERIFIED_BOUNDARY_DETAIL in crates/urollup-core/src/adapters/codex_rollout.rs) tells the user to inspect 'the rollout' and 'this thread', but report, daily and sessions summaries carry only code, count and detail (crates/urollup-core/src/query/aggregate.rs diagnostic_summaries; crates/urollup-core/src/query.rs DiagnosticSummary), diagnostics are subject-scoped, and no sessions row is marked partial (see tests/golden/e2e/codex-rollout/unverified-fork-boundary.tryscript.md, three rows and one aggregate x3 line). A user with many sessions cannot find the affected rollout except by running report --session on each. The detail also tells a legacy child without a boundary (copied counter, then an inconsistent first step; probe N4) to inspect subagent_history_start_ordinal, a field it does not have. Options: list the affected sessions' native IDs in this code's summary (sessions already prints them, so no new private data), or add a per-session coverage flag to sessions, possibly under the CLI-honesty work uro-oz6w; in every case reword the detail so it reads correctly with and without a boundary, without paths or values.
