# mixed-counter-direct

Tests a rollout that holds cumulative `token_count` turns before its first
`token_usage_record`: a session that a release before `rust-v0.153.0` started and a
later release resumed (design §3.4, uro-h2sf). Usage records account for a rollout’s
usage from its first one on; before it, the counter rules count each `token_count`
except a twin, the counter of a response that a usage record also reports.

- **Resumed session:** 0.150.0 writes two counter-only turns, three responses on lines
  5, 6 and 9. Three hours later 0.154.0 resumes it in the same file: in turn
  `turn-c17-m3` each response’s `token_count` (lines 13 and 15) comes just before its
  `token_usage_record` (lines 14 and 16). Line 13 precedes the first usage record, but
  its turn has usage records, so it is a twin and adds nothing; line 15 follows that
  record.
- **New session:** 0.154.0 starts it, and its first `token_count` (line 6) precedes the
  first usage record (line 7) of the same turn.
  Its only counter before that record is a twin, so it is accounted from its usage
  records alone.
- **Reconciled:** 7 requests and 139,700 tokens: the three counter responses and two
  usage records of the resumed session (115,500), and the new session’s two usage
  records (24,200).
- **Naive sums:** deciding counters or usage records for the whole file, the rule
  urollup used before uro-h2sf, drops the resumed session’s 49,700 pre-upgrade tokens (4
  requests, 90,000 tokens).
  Counting every `token_count` before the first usage record counts each twin a second
  time (9 requests, 180,000 tokens).
  Summing every `last_token_usage` agrees with the reconciled total here, since each
  response has one `token_count` and no snapshot repeats.
- **Shapes:** the
  [`token_count` event](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2317-L2321)
  and
  [TokenUsageRecord](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2237-L2248)
  follow Codex `rust-v0.154.0`, and the resumed `thread_token_usage` continues the
  restored running total.
  A response’s `token_count` written just before its `token_usage_record` is a shape
  seen in local rollouts for a session’s first recorded response; this case uses that
  order for every response, and its values and IDs are synthetic.
  Whether a resumed rollout repeats `session_meta` or writes `thread_settings_applied`
  when resumed is unverified, and neither would change the accounting.
