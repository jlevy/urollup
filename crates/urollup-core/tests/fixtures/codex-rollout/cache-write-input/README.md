# cache-write-input

Tests that Codex uncached input excludes cache writes as well as cache reads, because
`input_tokens` includes both `cached_input_tokens` and `cache_write_input_tokens`
(design §4.1).

- **Usage records:** a root rollout written by 0.154.0 with three responses, two of
  which report cache writes (300 and 200). Each record is followed by its twin
  `token_count`, which adds nothing.
- **Cumulative counters:** a root rollout written by 0.150.0 with no usage records; its
  totals report 200 and then 500 cache writes, so the second request’s delta has 300.
- **Reconciled:** 5 requests: 1,000 uncached input, 2,600 cache reads, 1,000 cache
  writes, 180 output (15 of it reasoning) and 4,780 tokens, equal to the native
  `total_tokens` of the three records plus the counter rollout’s last total.
  Inclusive input, 4,600, equals the native `input_tokens`.
- **Naive:** subtracting only cache reads, as urollup did before this case, leaves every
  cache-write token in uncached input as well: 2,000 uncached input and 5,780 tokens,
  1,000 over the native totals.
  ccusage 20.0.20 ignores `cache_write_input_tokens`: its totals match, but it reports
  the 1,000 cache-write tokens as uncached input and no cache writes (the parity
  ledger’s `codex-cache-write-input` entry).
- **Evidence:** Codex copies `input_tokens` and both cache counts from the Responses API
  usage verbatim, the cache counts from `input_tokens_details`, and already treats the
  cached count as a subset of `input_tokens`
  ([source review](../../../../../../docs/project/research/research-2026-09-14-agent-tool-source-reviews.md#codex-token-usage)).
  Pi’s Responses mapping subtracts both counts from `input_tokens`, and ccusage models
  writes inside input from commit
  [`15b3bef`](https://github.com/ccusage/ccusage/commit/15b3bef85b1e0d440ca98e345b4fb5610a41a195).
  Codex source does not state the inclusion, so the case pins an inference.
- **Shapes:** the `auto-review-model` and `counter-reset-epoch` cases’ records, with
  nonzero `cache_write_input_tokens`.
