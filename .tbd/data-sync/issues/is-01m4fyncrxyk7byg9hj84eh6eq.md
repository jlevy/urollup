---
type: is
id: is-01m4fyncrxyk7byg9hj84eh6eq
title: Report diagnostic rows keep one adapter's detail when Claude Code and Codex both report a code
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
created_at: 2026-10-09T09:07:17.404Z
updated_at: 2026-10-09T09:07:17.404Z
---
Residual of PR #18 review D2 (https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467941265). Since PR #18 commit fixing D2, each adapter emits one source-incomplete diagnostic whose detail names every loss kind with its count, for example '3 Codex rollouts could not be read completely: corrupt-compressed-data (2), incomplete-compressed-frame (1)'. The report's per-code row (query/aggregate.rs diagnostic_summaries) sums occurrences across both agents' ledgers but keeps only the first diagnostic's detail in canonical order, so with one damaged Claude transcript and three damaged Codex rollouts the row reads count 4 with detail 'a Claude Code transcript could not be read completely: incomplete-compressed-frame'. Every code is aggregated this way (DiagnosticSummary.detail is documented as the first diagnostic's detail). Fixing it needs a decision on the summary contract: join distinct details per code, emit a row per agent, or word details as samples. Waits on that decision.
