# brief-repeated-snapshot

Reproduces the Codex half of the research brief’s
[synthetic double-counting example](../../../../../../docs/project/research/research-2026-09-13-portable-agent-usage.md#synthetic-double-counting-example):
running totals of 10,000, 25,000 and 25,000 tokens.

- **Layout:** a 0.150.0 legacy rollout, before `token_usage_record`. Event 1 (line 5)
  has total and last 10,000; event 2 (line 6) has last 15,000 and total 25,000; event 3
  (line 7) repeats event 2 exactly, as a rate-limit update does.
- **Reconciled:** 2 requests and 25,000 tokens.
  `last_token_usage` counts only when the running total advances (design §3.4), so line
  7 is extra evidence for line 6. The repeated rate-limit snapshot is one observation
  per window (§3.1).
- **Naive sums:** summing running totals gives 60,000 tokens, and summing every
  `last_token_usage` gives 40,000.
- **Shapes:** `token_count` and `rate_limits` from Codex
  [protocol.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2215-L2391)
  and the
  [rollout decoder test](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/tests.rs#L52-L74);
  the rule from ccusage’s
  [Codex parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L320-L367).
