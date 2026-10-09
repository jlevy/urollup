---
type: is
id: is-01m4fvzqfvhw9qqj85jsbf7cyw
title: "PR #16 A6: regression pinned only by Rust tests with non-wire-shaped direct records"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:30.330Z
updated_at: 2026-10-09T09:05:34.393Z
closed_at: 2026-10-09T09:05:34.392Z
close_reason: "fixed in 686e7b0 and 5d8cdeb: direct() records carry thread_id; codex-rollout/paginated-counter-prefix fixture with expected.json, golden, e2e and parity; CLI exit pinned. Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6077860808"
resolution: null
duplicate_of: null
---
Low. Review A https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467191112. paginated_forks.rs:93-102 direct() omits thread_id; no fixture/expected.json/golden for the uro-kpbp shape; no CLI test of the boundary failure behavior.
