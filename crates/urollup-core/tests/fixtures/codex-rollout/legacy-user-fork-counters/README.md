# legacy-user-fork-counters

Tests a legacy user fork before `token_usage_record`: copied `token_count` events add
nothing, the fork’s counter continues the parent’s total, and the copied prefix is
labeled inferred (design §3.4).

- **Parent:** turn `turn-c08-p1` with two responses; the running total ends at 23,750.
- **Fork:** its own `session_meta` with `forked_from_id`, the parent’s seven lines
  copied with 14:20 write times, then its own turn, whose one event reports a 13,600
  last usage on a running total of 37,350.
- **Reconciled:** 3 requests: 2 owned by the parent and 1 by the fork, 37,350 tokens.
  The prefix boundary comes from the parent’s turn ID, with
  `codex-copied-history-inferred`.
- **Naive sum:** every `last_token_usage` in both files gives 5 requests and 61,100
  tokens.
- **Shapes:** Codex
  [copied forks](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/thread_manager.rs#L1332-L1360)
  and
  [counter seeding](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1486-L1493);
  ccusage’s
  [copied branch test](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L445-L519).
