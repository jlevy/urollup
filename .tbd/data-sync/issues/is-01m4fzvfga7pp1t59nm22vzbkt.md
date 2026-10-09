---
type: is
id: is-01m4fzvfga7pp1t59nm22vzbkt
title: "PR #16 C3: unpositioned token_count repeating the running total is reported as unverified usage"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fzvd803vx5er2c7k9cycgn
created_at: 2026-10-09T09:28:05.383Z
updated_at: 2026-10-09T09:47:40.475Z
closed_at: 2026-10-09T09:47:40.474Z
close_reason: "fixed in f456852: an unpositioned token_count repeating the running total carries no usage (no diagnostic, gap or counter rebase); a repeated first step defers the check. Confirmed by an_unpositioned_token_count_repeating_the_running_total_adds_nothing_and_is_no_gap and a_repeated_inherited_total_does_not_use_up_the_first_step_check (both red at 5d8cdeb); design line 865 updated. Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6078490540"
resolution: null
duplicate_of: null
---
Low. Review C https://github.com/jlevy/urollup/pull/16#pullrequestreview-5468251421. codex_rollout.rs:1135-1137, :1143, :1250-1254. Probe N1: after the child's first step 108/12, an unpositioned token_count repeating 108/12 (last 18/2), then 113/13 last 5/1 at ordinal 5: counted [6, 20, 100] correctly but a codex-history-boundary-unverified diagnostic and a gap make coverage Partial. Codex re-sends info on every rate-limit refresh (rust-v0.154.0 core/src/session/mod.rs:4487-4528). Design line 865 holds only for info: null.
