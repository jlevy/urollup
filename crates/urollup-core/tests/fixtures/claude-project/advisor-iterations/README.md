# advisor-iterations

Tests the design §4.1 rule for `iterations`: top-level `message.usage` equals the sum of
the `message` iterations and excludes `advisor_message` iterations, which record their
own model and are further model usage inside the same request.

- **Layout:** response `G1` carries `advisorModel` and three iterations: two `message`
  items whose counts sum to the top level (2 input, 530 output, 8,100 cache write,
  240,000 cache read) and one `advisor_message` with `claude-opus-4-5`. Response `G2`
  has a single `message` iteration equal to its top level.
- **Reconciled:** 2 requests (2 calls) with three model usage rows; `G1` has 150,002
  input and 7,730 output tokens across two models, priced separately.
- **Naive sums:** top level alone loses 150,000 advisor input tokens; adding every
  iteration to the top level counts the message iterations twice (496,200 cache read);
  one row per advisor iteration gets tokens right but reports 3 calls.
- **Shapes:** adapted from ccusage’s
  [advisor fixture](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L319-L369)
  (MIT), with new synthetic values.
