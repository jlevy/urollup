# compaction-lowered-total

Tests that a cumulative `token_count` total that Codex lowers after a `compacted`
record, without restarting it from zero, opens a new counter epoch with a diagnostic,
and that the decreasing record counts only its own `last_token_usage`, not the new total
(design §3.4, uro-v1c9).

- **Layout:** turn 1 accumulates to 92,500 tokens over lines 4–5. Turn 2 advances to
  214,500 (line 9), compacts (line 10), and reports a lowered total of 155,100 whose
  `last_token_usage` is 9,600 (line 11): uncached input and cache reads fall while
  output and reasoning keep growing.
  Line 12 then advances by 40,900 from the lowered total.
- **Reconciled:** 5 requests and 265,000 tokens.
  Line 11 opens epoch 2 with `codex-counter-epoch-reset` and counts its
  `last_token_usage` (9,600). Line 12’s delta, taken from line 11’s total in the same
  epoch, equals its own `last_token_usage`.
- **Naive sums:** charging the decreasing total in full to its request, the rule urollup
  used before uro-v1c9, counts line 11 as 155,100 tokens and the case as 410,500.
  Codex’s own last running total, 196,000, understates the case by the 69,000 tokens the
  compaction removed. Summing every `last_token_usage` agrees with the reconciled 265,000
  here, since no snapshot repeats and no estimate appears.
- **Contrast:** in `counter-reset-epoch` the counter restarts from zero, so the new
  total equals `last_token_usage` and both rules agree; here the new total is more than
  16 times the request’s own usage, so the rule for the decreasing record decides the
  result.
- **Shapes:** the
  [`token_count` event](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2317-L2321)
  and the
  [`compacted` item](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/rollout_payload.rs#L154-L180)
  follow Codex `rust-v0.154.0`. That release
  [adds each response’s usage to the total](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2259-L2289)
  and lowers it only for a context-window fill, which zeroes every component.
  A total lowered to a nonzero value next to a `compacted` record is what uro-v1c9
  reports from recent local rollouts; which releases write it is unverified, so this
  models a shape with synthetic values, not a known release behavior.
