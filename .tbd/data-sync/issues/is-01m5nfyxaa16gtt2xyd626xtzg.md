---
type: is
id: is-01m5nfyxaa16gtt2xyd626xtzg
title: Exclude implausible token counts instead of aborting on overflow
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:29.033Z
updated_at: 2026-10-23T23:01:29.033Z
---
Any token value near u64::MAX (or two values near 2^63) makes report, daily and sessions exit 1 with 'token counter overflow' for both agents (aggregate.rs:40 and 239, TokenCounts::from_measures at query.rs:175-188; Codex checks at codex_rollout.rs:2074 and 2124). This is the failure class uro-381f fixed for input underflow. Fix: adapters reject counts above a plausibility bound (for example 2^53 per field) as inconsistent usage with a diagnostic and coverage gap, as codex-inclusive-input-underflow does; aggregation keeps checked arithmetic.
