# zst-twin

Tests that a Codex `.jsonl` rollout and its `.jsonl.zst` twin with the same thread and
rollout ID are one logical source whose representation changed, not two sources (design
§2.2).

- **Layout:** in one date directory,
  `rollout-…-019f0000-0000-7000-8000-001000000001.jsonl.zst` holds 10 lines with records
  `1` and `2`. The plain `.jsonl` beside it holds the same 10 lines plus a turn appended
  a week later with record `3`, as a resume does after decompressing.
- **Reconciled:** 3 requests and 25,070 tokens, with records `1` and `2` evidenced in
  both representations.
- **Naive sum:** treating each physical file as a source counts records `1` and `2`
  twice: 5 requests and 40,840 tokens.
- **Shapes:** Codex
  [compression worker](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L610-L679),
  [plain-over-compressed discovery](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L144-L192)
  and
  [decompress on resume](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L73-L123).
