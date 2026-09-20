# missing-request-id

Tests that a missing `requestId` does not prevent grouping: the `req-` key uses the
response ID (`message.id`) first (design §3.6), and a message ID that does not start
with `msg_01` is still a native response ID.

- **Layout:** response `msg_bdrk_01Synthetic09A` is written as two block records with
  `output_tokens` 30 and 95 and no `requestId`; response `msg_bdrk_01Synthetic09B` is
  one record.
- **Reconciled:** 2 requests with native identity, `A` from line 3.
- **Naive sum:** a reader that deduplicates by `requestId` (session-report keys on it)
  cannot group the blocks: 3 requests and 147 output tokens against 2 and 117.
- **Shapes:** ccusage’s
  [requestless duplicate test](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L281-L301).
  The `msg_bdrk_` prefix is a modeled Bedrock-style ID; which gateways and providers
  omit `requestId` is unverified.
