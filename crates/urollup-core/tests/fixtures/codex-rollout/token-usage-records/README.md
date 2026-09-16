# token-usage-records

Tests the `rust-v0.153.0` and later request rule: each `token_usage_record` is one
response keyed by `response_id` and owned by its `thread_id`, a record whose `thread_id`
differs from the file’s is a copy, and `compacted.latest_token_usage_record` is never an
observation (design §3.4).

- **Parent:** records `p1` and `p2` in turn 1, then a resumed turn 2 two hours later in
  the same file with compaction response `p3`, whose `turn_token_usage` restarts while
  `thread_token_usage` continues.
  Line 15 is a `compacted` item carrying a copy of the `p3` record.
- **Legacy user fork:** its own `session_meta` with `forked_from_id`, the parent’s first
  ten lines copied with new timestamps (records still naming the parent’s thread), its
  own `thread_settings_applied` boundary, and record `f1`.
- **Reconciled:** 4 requests: `p1`, `p2` and `p3` owned by the parent, `f1` by the fork,
  and 73,200 tokens.
- **Naive sums:** every record line plus the compacted copy gives 7 requests and 119,600
  tokens; summing each rollout’s final `thread_token_usage` gives 99,100.
- **Shapes:** Codex
  [TokenUsageRecord](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2237-L2248),
  [wire shapes](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/tests.rs#L393-L518),
  [records across resume](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/token_usage_rollout.rs#L31-L116)
  and
  [copied user forks](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/thread_manager.rs#L1332-L1397).
