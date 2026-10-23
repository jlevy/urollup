---
type: is
id: is-01m5nfy00szzk1qqk1dywg67gy
title: Count a Codex token_count delta when last_token_usage is absent
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - correctness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:59.032Z
updated_at: 2026-10-23T23:00:59.032Z
---
At codex_rollout.rs:2345 a token_count with a total but no last_token_usage key advances the tracker without emitting an observation, so that step's usage is folded into the running total and lost (synthetic 2,200 instead of 3,300, complete), while last_token_usage: null counts the delta. Drift-only: current Codex writes both fields. Fix: derive the step from the total delta as the null case does.
