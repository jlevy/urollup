---
type: is
id: is-01m4fvzq1rqzm30s93md68qjjk
title: "PR #16 A5: comment says the explicit boundary establishes the inherited baseline"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:29.879Z
updated_at: 2026-10-09T09:05:34.096Z
closed_at: 2026-10-09T09:05:34.095Z
close_reason: "fixed in 686e7b0: comment now says the boundary establishes prefix ownership, not a counter baseline. Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6077860808"
resolution: null
duplicate_of: null
---
Low. Review A https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467191112. codex_rollout.rs:1004-1005 contradicts docs/urollup-design.md:881-883 and :1127-1133.
