# mixed-counter-direct

Tests a root rollout that holds cumulative `token_count` usage beside
`token_usage_record` lines: a session that a release before `rust-v0.153.0` started and
a later release resumed (design §3.4, uro-h2sf). Both sessions are root rollouts, with
one `session_meta` each and no parent, fork origin or history boundary.
A `token_count` is the twin of a usage record when the two are adjacent usage events
with exactly the same usage, and adds nothing but the running total; every other
`token_count` that reports usage goes through the counter rules.

- **Resumed session:** 0.150.0 writes two counter-only turns, three responses on lines
  5, 6 and 9. Three hours later 0.154.0 resumes it in the same file: in turn
  `turn-c17-m3` each response’s `token_usage_record` (lines 13 and 15) comes just before
  its `token_count` (lines 14 and 16), which are twins.
- **New session:** 0.154.0 starts it; each `token_count` (lines 7 and 9) is the twin of
  the usage record just before it (lines 6 and 8), so the session is accounted from its
  usage records alone, as before uro-h2sf.
- **Reconciled:** 7 requests and 139,700 tokens: the three counter responses and two
  usage records of the resumed session (115,500), and the new session’s two usage
  records (24,200).
- **Naive sums:** deciding counters or usage records for the whole file, the rule
  urollup used before uro-h2sf, drops the resumed session’s 49,700 pre-upgrade tokens (4
  requests, 90,000 tokens).
  Counting every `token_count` and every usage record counts each response 0.154.0 wrote
  twice (11 requests, 229,700 tokens).
  Summing every `last_token_usage` agrees with the reconciled total here, since each
  response has one `token_count` and no snapshot repeats.
- **Shapes:** the
  [`token_count` event](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2317-L2321)
  and
  [TokenUsageRecord](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2237-L2248)
  follow Codex `rust-v0.154.0`, which persists a response’s usage record when the
  response completes and sends its `token_count` afterwards.
  On resume, 0.154.0 seeds a session’s usage-record totals from the rollout’s last usage
  record; this rollout had none, so the resumed records’ `thread_token_usage` counts
  from the first resumed response.
  urollup reads neither `thread_token_usage` nor `turn_token_usage`. The reverse order,
  a `token_count` just before its record, is covered by
  `crates/urollup-core/tests/codex_mixed_usage.rs`. Values and IDs are synthetic.
  Whether a resumed rollout repeats `session_meta` or writes `thread_settings_applied`
  when resumed is unverified, and neither would change the accounting.
