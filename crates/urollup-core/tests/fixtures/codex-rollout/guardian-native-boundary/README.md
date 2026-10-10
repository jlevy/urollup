# guardian-native-boundary

Tests that the boundary of a natively created paginated Guardian review still assigns
its inherited prefix to other threads (design §3.4), including an earlier review’s
checkpoint that carries usage.

- **Parent:** a paginated root with one record, `p1`.
- **First review:** created from the parent’s compaction.
  Its prefix is that compaction item at ordinal 1, `subagent_history_start_ordinal` is
  2, and its own `thread_settings_applied` sits at the boundary before turn `a1`.
- **Second review:** created from the first review’s checkpoint.
  Its prefix holds a `compacted` line keeping `a1`’s record, the first review’s turn
  context and its running total, whose last usage is `a1`. The boundary is 4, and its
  own settings and turn `b1` follow.
- **Reconciled:** 3 requests and 18,900 tokens, one per thread.
  The checkpoint’s record names the first review, and its counter follows that record,
  so both are copies of `a1`.
- **Naive:** counting every `token_count`’s last usage in the rollout that holds it
  counts `a1` twice: 4 requests and 21,590 tokens.
- **Shapes:** Codex
  [native child boundary](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/live_thread.rs#L155-L177),
  [settings at the boundary](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/mod.rs#L1704-L1716),
  [Guardian fork from a compaction](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/guardian/review_session_setup.rs#L122-L134)
  and
  [Guardian checkpoint](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/guardian_checkpoint.rs#L11-L41);
  see
  [the boundary research note](../../../../../../docs/project/research/research-2026-10-10-codex-paginated-subagent-boundary.md).
