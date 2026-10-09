---
type: is
id: is-01m4fvzs4atmmtb89aq7esz4eg
title: "PR #16 B4: design doc states the counter baseline rule more broadly than the code"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:32.008Z
updated_at: 2026-10-09T09:05:35.591Z
closed_at: 2026-10-09T09:05:35.590Z
close_reason: "fixed in 686e7b0: design §3.4 scopes the baseline check to explicit-boundary children and children after copied counters. Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6077861115"
resolution: null
duplicate_of: null
---
Low. Review B https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389. docs/urollup-design.md:866-868 vs codex_rollout.rs:1127-1130 (check runs only for explicit boundaries > 0).
