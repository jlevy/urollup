# info-null

Tests that `token_count` with `info: null` is only a provider limit observation and that
a rate-limit-only update with unchanged `info` adds no usage (design §3.4).

- **Layout:** line 4 is `info: null` with a rate-limit snapshot, as Codex writes before
  a thread’s first recorded usage; line 5 is the first response (5,400 tokens) with the
  same snapshot; line 6 repeats line 5’s `info` with a new snapshot (primary 42%).
- **Reconciled:** 1 request and 5,400 tokens.
  Snapshots on lines 4 and 5 are identical, so they are one observation per window; line
  6 is a second one.
- **Naive sum:** squares counts all three events as responses and adds two
  `last_token_usage` values: 3 responses and 10,800 tokens.
- **Shapes:** Codex
  [turn.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L1484-L1494)
  and
  [session tests](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/tests.rs#L2808-L2936);
  the squares probe from its
  [rollup reader](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L278-L290).
