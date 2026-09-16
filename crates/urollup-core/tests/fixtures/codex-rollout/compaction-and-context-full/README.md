# compaction-and-context-full

Tests that compaction estimates and context-window-full fills, both with zero input and
output and nonzero `total_tokens`, are estimate diagnostics, never usage, and that the
fill opens a new counter epoch (design §3.4).

- **Layout:** response `A` (line 4), a `compacted` item and an estimate whose
  `last_token_usage` holds only 3,200 total tokens (line 6), response `B` (line 7), a
  fill on line 11 that sets the running total to the 272,000-token window with zero
  components, a failed turn, and response `C` (line 15) counting up from zero
  components.
- **Reconciled:** 3 requests and 194,400 tokens, with estimate diagnostics on lines 6
  and 11 and an epoch reset on line 11.
- **Naive sums:** summing every `last_token_usage` gives 316,100 total tokens; the last
  running total, which Codex stores as `tokens_used`, is 312,900.
- **Shapes:** Codex
  [estimates and fills](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4447-L4536),
  [fill_to_context_window](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2291-L2314)
  and the
  [compaction test](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/compact.rs#L990-L1078).
