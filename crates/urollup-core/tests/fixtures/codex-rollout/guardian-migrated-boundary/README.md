# guardian-migrated-boundary

Tests that a usage record naming a rollout’s own thread is that thread’s request even
before `subagent_history_start_ordinal` (design §3.4). Codex’s legacy-to-paginated
migration sets the boundary one past the last line it rewrote, so a migrated Guardian
review holds its own records before it.

- **Parent:** a paginated root with one record, `p1`.
- **First review:** a `guardian_review` rollout migrated whole.
  It holds the parent’s compaction item, its own `thread_settings_applied`, and two
  turns with records `g1` and `g2`. Its boundary is 15 and its last ordinal 14.
- **Second review:** migrated from its own newest compaction.
  Only a `compacted` line keeping `h1`’s record and one turn with record `h2` survive,
  every line stamped with the `session_meta` time.
  Its boundary is 6 and its last ordinal 5.
- **Reconciled:** 4 requests and 17,040 tokens: `p1`, `g1`, `g2` and `h2`, each owned by
  the thread its record names.
  `h1` is a copy-only request, because the migration dropped its original line.
- **Naive:** treating every line before the boundary as the parent’s copy, as urollup
  did before `uro-jqc3`, finds only `p1`: 1 request and 9,600 tokens, with coverage
  still complete. Summing each rollout’s running total gives 21,190 tokens.
- **Shapes:** Codex
  [migration boundary](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L714-L731),
  [boundary rewrite](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/publish.rs#L156-L178),
  [dropped headers](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L82-L117)
  and
  [bounded suffix](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L874-L914);
  see
  [the boundary research note](../../../../../../docs/project/research/research-2026-10-10-codex-paginated-subagent-boundary.md).
