# cache-creation-breakdown

Tests the design §4.1 rule that when Claude’s `cache_creation` breakdown (5-minute and
1-hour writes) disagrees with `cache_creation_input_tokens`, both native values are kept
with a diagnostic.

- **Layout:** `M1` has a flat count of 5,000 but a breakdown of 3,000 plus 2,500; `M2`
  has a consistent all-1-hour breakdown of 4,000; `M3` has only the flat count of 700.
- **Reconciled:** 3 requests and one `claude-cache-creation-breakdown-mismatch`
  diagnostic on `M1`. The cache-write total uses the flat counts (9,700), and
  `cache_write_by_duration` reports 3,000 5-minute, 6,500 1-hour and 700 unrecorded
  writes. Which value feeds the total on disagreement is an open question.
- **Naive sums:** preferring the breakdown (ccusage) gives 10,200 cache writes; using
  only the flat count gets tokens right but prices 1-hour writes at the 5-minute rate.
- **Shapes:** ccusage’s
  [cache-write accessor](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/types.rs#L28-L57)
  and the receipts miner’s
  [duration weights](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L170-L212).
