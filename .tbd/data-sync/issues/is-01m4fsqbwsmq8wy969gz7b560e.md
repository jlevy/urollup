---
type: is
id: is-01m4fsqbwsmq8wy969gz7b560e
title: Codex rollouts with any token_usage_record drop token_count-only usage
kind: bug
status: in_progress
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T07:40:59.160Z
updated_at: 2026-10-09T19:16:55.827Z
---
Found by PR #16 correctness review B (B5), outside that layer's diff and present at main: in codex_rollout.rs the presence of any token_usage_record in a rollout switches the whole file to the direct path (has_direct), and token_count counter records are then ignored. A session started before Codex 0.153 (counter-only) and resumed after it (direct records) would silently lose its pre-upgrade usage while coverage reports complete. Synthetic reproduction in the review counted 22 of 77 tokens. The shape is inferred from the research brief and not yet seen in a real log. Decide per region (counter records before the first direct record, or per turn) and add a fixture; if ownership cannot be established, report partial coverage with a diagnostic rather than dropping usage. Review: https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389

## Notes

Deferred from PR #16 review B (B5) in the disposition reply https://github.com/jlevy/urollup/pull/16#issuecomment-6077861115; waits on a per-region decision for rollouts mixing token_count-only usage with token_usage_record lines, plus a fixture.

Fix in review: https://github.com/jlevy/urollup/pull/26 (branch fix/codex-mixed-format-usage, commits eea914a and bb96642). Counters before a rollout's first token_usage_record go through the existing counter rules, except a twin (its turn has a usage record, or with no turn ID its last_token_usage equals the first record's usage), which still advances the running total; counters at or after the first record stay ignored. Fixture codex-rollout/mixed-counter-direct and tests in crates/urollup-core/tests/codex_mixed_usage.rs. Close on merge.
