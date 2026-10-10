---
type: is
id: is-01m4fsqbwsmq8wy969gz7b560e
title: Codex rollouts with any token_usage_record drop token_count-only usage
kind: bug
status: closed
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T07:40:59.160Z
updated_at: 2026-10-10T17:13:42.915Z
closed_at: 2026-10-10T17:13:42.914Z
close_reason: "Merged in PR #26 (a4bad34): root rollouts count counter-only turns beside usage records by an adjacency twin rule; forks and subagents keep the gap (uro-r8si)."
resolution: null
duplicate_of: null
---
Found by PR #16 correctness review B (B5), outside that layer's diff and present at main: in codex_rollout.rs the presence of any token_usage_record in a rollout switches the whole file to the direct path (has_direct), and token_count counter records are then ignored. A session started before Codex 0.153 (counter-only) and resumed after it (direct records) would silently lose its pre-upgrade usage while coverage reports complete. Synthetic reproduction in the review counted 22 of 77 tokens. The shape is inferred from the research brief and not yet seen in a real log. Decide per region (counter records before the first direct record, or per turn) and add a fixture; if ownership cannot be established, report partial coverage with a diagnostic rather than dropping usage. Review: https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389

## Notes

2026-10-09 PR #26 round 1 (reviews A and B): the region cut at the first usage record still dropped counter-only turns after it (older client resuming a newer file), and the turn-keyed twin test could double count a counter-first compaction or drop an equal turn-less response. A local scan of real history (qualitative only) found record-first order everywhere and no counter-first twin. Decision: one whole-rollout adjacency rule; a usage-advancing token_count is a twin only when the nearest usage event before or after it is a usage record with equal usage; every other counter goes through the counter rules wherever it is. Copied counters are keyed to their twin record's response ID. Fix in progress on PR #26.
