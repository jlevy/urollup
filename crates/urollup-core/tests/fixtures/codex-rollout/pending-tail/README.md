# pending-tail

Tests the design §2.2 snapshot boundary: an unfinished last line is pending, distinct
from interior corruption, and every malformed line is counted per source.

- **Layout:** records `1` and `2`, with a torn `event_msg` on line 8 between them, and a
  `token_usage_record` for response `3` cut off mid-object on line 11 with no trailing
  newline.
- **Reconciled:** 2 requests and 7,270 tokens, one `malformed-line` diagnostic for line
  8 and one `pending-tail` for line 11. Response `3` is not counted until its line
  completes.
- **Naive:** a strict reader loses the whole rollout; a silent lenient reader gets the
  tokens but reports no coverage problem.
- **Decode declarations:** `expected.json` lists line 8 under `decode.malformed` and
  line 11 under `decode.pending_tail`, which `scripts/check-fixtures.mjs` requires.
- **Shapes:** Codex
  [skips malformed lines](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1026-L1089)
  and
  [tests a truncated tail](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder_tests.rs#L1001-L1174).
