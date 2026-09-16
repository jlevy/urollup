# nested-null-tool-input

A negative case for ccusage 20.0.20, which rejects any Claude line containing
`"id":null`, `"model":null` and similar names at any depth.
Design §2.2 says a record is never rejected because a nested field is null.

- **Layout:** response `H1` has a text block (`output_tokens` 8) and an `Edit` block
  (150) whose input nests `id`, `model`, `cwd` and `version` as null.
  Response `H2` is a `Write` whose input nests `speed`, `costUSD`, `sessionId` and
  `requestId` as null.
- **Reconciled:** 2 requests, `H1` from line 3, 8 input and 220 output tokens.
- **Naive sums:** ccusage’s null filter drops lines 3 and 5, so `H1` counts through its
  smaller block record and `H2` disappears: 6 input and 8 output tokens.
- **Shapes:** the filter is
  [`has_unsupported_null_field`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L455-L500);
  its
  [tests](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L638-L656)
  are adapted here as a negative case.
