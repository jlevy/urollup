---
type: is
id: is-01m4fzvz3amsqnkzs9mkj0z9ag
title: "Codex: a child-owned usage record before the fork boundary is excluded silently with complete coverage"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T09:28:21.351Z
updated_at: 2026-10-11T20:09:19.331Z
closed_at: 2026-10-11T20:09:19.331Z
close_reason: "Resolved by PR #32 (merged 2026-10-10, main 0f0f6df): a token_usage_record that names the rollout's own thread is that thread's original wherever it sits, before a declared boundary included (design §3.4). Probe N2 on main df8c5ac (boundary 3, parent record at 1, child-named record at 2, own record at 3) now counts [20, 33, 100] with coverage complete, no copy-only request, no diagnostic or gap. Committed coverage of the rule is a_record_naming_its_own_thread_counts_without_a_placeable_position (unpositioned and invalid boundaries); the exact placed-boundary N2 shape is not separately pinned."
resolution: null
duplicate_of: null
---
Found by PR #16 follow-up review C (C2, Low): https://github.com/jlevy/urollup/pull/16#pullrequestreview-5468251421. In crates/urollup-core/src/adapters/codex_rollout.rs (observe_parsed_source), a token_usage_record before an explicit subagent_history_start_ordinal whose own thread_id names the rollout's thread (claimed_by_file is true) is classified as a copy because before_boundary wins, and only records the boundary cannot place (no ordinal, invalid boundary) feed the codex-history-boundary-unverified diagnostic and coverage gap. Probe N2 (synthetic): direct-usage child, boundary 3, parent record at ordinal 1, a token_usage_record with the child's thread_id at ordinal 2 (30/3), the child's own record at ordinal 3 (18/2): counted [20, 100]; the 33 tokens become a copy-only request, coverage stays Complete and no diagnostic is emitted. This contradicts design §3.4 (docs/urollup-design.md, Codex copied history), which says usage a declared boundary cannot place is reported with one diagnostic and a gap per rollout. Reach: no known Codex writer produces it (0.153+ drops token_usage_record from forked prefixes; Codex rollout/src/ordinal.rs:86-96 refuses incomplete prefixes). Suggested fix: when claimed_by_file && before_boundary, keep the record excluded but add it to the rollout's unverified evidence so it gets the diagnostic and the coverage gap; add N2 as a test in crates/urollup-core/tests/paginated_forks.rs.
