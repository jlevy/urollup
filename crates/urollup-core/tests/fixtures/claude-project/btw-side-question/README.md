# btw-side-question

Tests the design §3.4 rule that a `/btw` side-question record with the parent’s
`message.id` under a new `requestId` is a copy owned by the parent, not a new request or
an ambiguous key.

- **Layout:** the parent writes response `P` as two block records (lines 2–3) and a
  second response `Q`. The side-question transcript under `subagents/` is marked
  `isSidechain`; its line 1 replays `P` with a new `requestId` and 50,000 cache-read
  tokens, then it asks and answers its own question.
- **Reconciled:** 3 requests: `P` from parent line 3 (largest `output_tokens`), `Q`, and
  the side answer owned by the side-question thread.
  The replay adds no usage.
- **Order independence:** read the side-question file first too; ccusage 20.0.20’s
  `daily` loader double counted when a replay preceded its parent’s block records.
- **Naive sums:** every record gives 105,100 cache-read tokens; keying on
  `(message.id, requestId)` still counts the replay, giving 87,100 against 37,100.
- **Shapes:** ccusage’s
  [sidechain notes](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/README.md#L19-L31)
  and
  [replay tests](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L690-L782).
  The `aside_question` file label is inferred and unverified.
