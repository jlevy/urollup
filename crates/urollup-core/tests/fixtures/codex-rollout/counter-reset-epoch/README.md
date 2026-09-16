# counter-reset-epoch

Tests that a decrease in any cumulative `token_count` component opens a new counter
epoch with a diagnostic, and that deltas are taken only within an epoch (design §3.4).

- **Layout:** turn 1 accumulates to 7,800 tokens over lines 4–5. Turn 2, two hours
  later, starts over at 2,250 (line 9), grows to 4,900 (line 10), and repeats that total
  (line 11).
- **Reconciled:** 4 requests and 12,700 tokens.
  Line 9 opens epoch 2 with `codex-counter-epoch-reset` and counts its
  `last_token_usage`, which equals its new total; line 11 adds nothing.
- **Naive sums:** summing every `last_token_usage` counts the repeat (15,350 tokens); a
  total-only reader taking saturating differences turns the reset into zeros and loses
  line 9 (10,450).
- **Shapes:** ccusage’s
  [total-only path](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L1063-L1084)
  shows the saturating failure.
  Codex seeds resumed totals from the file
  ([session/mod.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1464-L1471)),
  so this models a shape rather than a known release behavior.
