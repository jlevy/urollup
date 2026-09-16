# multi-limit-id

Tests that Codex `rate_limits` snapshots become provider limit observations keyed by
`limit_id` and window, with native values verbatim, identical consecutive snapshots
merged, and carried-forward fields marked as possibly stale (design §3.1, §4.4).

- **Layout:** three responses with `token_usage_record` lines.
  Their `token_count` events carry a `codex` snapshot (line 7), a `codex_other` snapshot
  with only a primary window (line 9) repeated exactly on line 10, and a newer `codex`
  snapshot (line 12). Every snapshot carries the same `plan_type` and `credits`.
- **Reconciled:** 3 requests and 15,450 tokens; 5 limit observations (2 + 1 + 2), each
  listing `credits`, `spend_control_reached` and `plan_type` as possibly stale.
- **Naive sum:** one observation per event gives 6, and treating each `token_count` as
  usage counts the line 10 repeat: 20,660 tokens.
- **Shapes:** Codex
  [RateLimitSnapshot](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2323-L2391),
  [limit_id headers](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/codex-api/src/rate_limits.rs#L23-L100)
  and
  [carried-forward fields](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L388-L411).
