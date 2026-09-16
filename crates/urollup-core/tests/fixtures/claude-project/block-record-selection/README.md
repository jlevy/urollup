# block-record-selection

Tests the design §3.4 rule for block records that disagree: select one whole record by
largest `output_tokens`, then last in file order, then lowest `src-` ID; diagnose input
or cache differences; never merge fields.

- **B1 (lines 2–4):** three blocks with `output_tokens` 5, 5 and 180 and identical input
  and cache fields. Line 4 wins, with no diagnostic, because only `output_tokens`
  differs.
- **B2 (lines 6–7):** the larger `output_tokens` (250) comes first and the blocks
  disagree on `cache_creation_input_tokens` (10 and 12). Line 6 wins whole, so cache
  writes are 10, and a `claude-block-usage-conflict` diagnostic names both lines.
- **B3 (lines 9–10):** `output_tokens` ties at 40 and `cache_read_input_tokens` differs,
  so the last record, line 10, wins with a diagnostic.
- **Naive sums:** summing every record gives 7 requests and 216,500 cache-read tokens; a
  field-wise maximum (the receipts miner) reports 1,212 cache writes, a combination no
  record held.
- **Shapes:** the disagreement rate and field-wise maximum come from the
  [receipts miner](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L412-L432);
  ccusage’s
  [most complete duplicate test](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L257-L279)
  keeps a whole record.
