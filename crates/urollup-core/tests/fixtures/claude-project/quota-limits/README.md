# quota-limits

Tests that Claude limit records are kept as provider limit observations with native
values verbatim (design §3.1, §4.4), and that a `<synthetic>` error record is a
synthetic event, not a provider request (§4.1 Calls).

- **Layout:** responses `K1` and `K2` carry `quotaLimits` objects for the `five_hour`
  limit with statuses `allowed` and `allowed_warning` and one `resetsAt` in epoch
  seconds. Line 6 is a `<synthetic>` record with `isApiErrorMessage: true`, zero usage
  and the text `Claude AI usage limit reached|<epoch>`.
- **Reconciled:** 2 requests and 1 synthetic event; 3 limit observations, each timed by
  its record and with no window length.
- **Naive sum:** counting every assistant record reports 3 requests.
- **Shapes:** the `quotaLimits` key set (`status`, `rateLimitType`, `resetsAt`,
  `isUsingOverage`, `overageStatus`, `overageDisabledReason` and
  `unifiedRateLimitFallbackAvailable`) and its `rejected`, `five_hour`, `seven_day` and
  `out_of_credits` values were observed in a Claude Code 2.1.270 session on 2026-09-15
  (see the fixtures README); the values here are invented, and the field remains
  undocumented. The error text follows ccusage’s
  [usage-limit parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L532-L557).
