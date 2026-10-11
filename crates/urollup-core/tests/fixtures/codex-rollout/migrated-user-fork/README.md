# migrated-user-fork

Tests a legacy user fork after Codex’s legacy-to-paginated migration (design §3.4,
`uro-p9ua`): its copied `token_count` events add nothing, and its own counter continues
the copied total. The pair is `codex-rollout/legacy-user-fork-counters` as
`codex migrate-rollouts --apply` rewrites it.

- **Parent:** a migrated root with turn `turn-c20-p1` and two responses; the running
  total ends at 23,750.
- **Fork:** its own `session_meta` at ordinal 0, rewritten as paginated with
  `forked_from_id` but no `subagent_history_start_ordinal` and no `history_base`. The
  migration dropped the parent’s copied `session_meta`, so the parent’s six lines (14:20
  write times) follow it with nothing naming them, then the fork’s own turn
  `turn-c20-f1`, whose one event reports a 13,600 last usage on a running total of
  37,350.
- **Reconciled:** 3 requests: 2 owned by the parent and 1 by the fork, 37,350 tokens,
  with complete coverage, as before the migration.
  The parent root recorded `turn-c20-p1`, so the copy ends at `turn-c20-f1`, the first
  turn it never recorded, and that turn’s step continues the copied 23,750 by exactly
  its own last usage. No diagnostic: unlike a copy that follows another thread’s
  `session_meta`, a turn inference for a migrated rollout emits no
  `codex-copied-history-inferred`.
- **Naive:** every `last_token_usage` in both files gives 5 requests and 61,100 tokens.
  urollup before `uro-p9ua` reported the same 61,100, with complete coverage, because
  the fork’s copied counters looked like its own.
- **Without the parent:** nothing decides which counters the fork copied, so its three
  steps become a `codex-history-boundary-unverified` coverage gap, never a count
  (`crates/urollup-core/tests/paginated_forks.rs`).
- **Shapes:** Codex’s migration
  [rewrites the head](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L82-L100)
  and
  [drops later headers](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L117),
  and gives only
  [subagent sources](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L458-L473)
  a boundary; see
  [the boundary research note](../../../../../../docs/project/research/research-2026-10-10-codex-paginated-subagent-boundary.md#migrated-legacy-user-forks).
