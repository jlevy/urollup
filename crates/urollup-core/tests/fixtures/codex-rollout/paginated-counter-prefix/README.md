# paginated-counter-prefix

Tests the `uro-kpbp` fork shape: a counter-only paginated subagent whose copied prefix
holds the parent’s `token_count` but no parent `session_meta`, so only
`subagent_history_start_ordinal` separates inherited usage from the child’s own (design
§3.4).

- **Parent:** a paginated rollout (ordinals 0–5) with one request, a 9,500-token
  cumulative total.
- **Child:** `session_meta` at ordinal 0 with `subagent_history_start_ordinal: 3`, the
  parent’s `token_count` and user message copied at ordinals 1–2 without the parent’s
  session header, then its own turn and one `token_count` with a 14,800-token total and
  a 5,300-token last usage.
- **Reconciled:** 2 requests, one per thread, and 14,800 tokens.
  The copied counter reconciles with the parent’s original and seeds the child’s
  inherited baseline, so the child’s first delta, 5,300, equals its last usage.
- **Naive sum:** treating the records before the boundary as the child’s own counts the
  copied counter as a third request: 24,300 tokens.
- **Shapes:** Codex
  [paginated subagent prefix](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/live_thread.rs#L115-L145),
  [prefix validation](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/ordinal.rs#L86-L96)
  and
  [counter seeding](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1486-L1493).
  Before 0.153 a paginated prefix keeps the `token_count` events that seed the child’s
  counter.
