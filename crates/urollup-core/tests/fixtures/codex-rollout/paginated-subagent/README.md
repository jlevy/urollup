# paginated-subagent

Tests the native child boundary: a paginated subagent’s own records start at
`subagent_history_start_ordinal` (design §3.4), so inherited records never become child
turns, times or usage.

- **Parent:** a paginated rollout (ordinals 0–7) with one record, `p1`.
- **Child:** `session_meta` at ordinal 0 with `subagent_history_start_ordinal: 5`, a
  copied prefix at ordinals 1–4 (the parent’s `session_meta`, turn context,
  `task_started` and user message, restamped), then its own settings, turn with
  `root_turn_id`, and record `c1`.
- **Reconciled:** 2 requests, one per thread, and 14,800 tokens; `c1` links to the
  parent’s root turn.
- **Naive sum:** each rollout’s final running total counts the parent’s usage twice:
  24,300 tokens.
- **Shapes:** Codex
  [paginated subagent prefix](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/live_thread.rs#L115-L145),
  [prefix validation](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/ordinal.rs#L86-L96),
  [paginated copy filtering](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105)
  and
  [recorder tests](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder_tests.rs#L1001-L1174).
