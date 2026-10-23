---
type: is
id: is-01m5nfxzab58df2dg4am0kst14
title: Pair Codex usage-record twins with missing-as-zero counts
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
created_at: 2026-10-23T23:00:58.315Z
updated_at: 2026-10-23T23:00:58.315Z
---
Twin pairing at codex_rollout.rs:1218 and 1239 compares Option counts strictly, while same_counts and total_digest (489-497) read missing as 0. A token_usage_record without cache_write_input_tokens beside its counter with 0 is not paired (synthetic 5,500 instead of 3,300, complete). Low realism because 0.153+ serializes both lines from one struct. Fix: use same_counts.
