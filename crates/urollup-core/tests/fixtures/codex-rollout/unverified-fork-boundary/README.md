# unverified-fork-boundary

Tests the per-rollout degrade for fork-boundary evidence that cannot separate inherited
usage from a child’s own (design §3.4): the unproven usage is excluded, never counted,
each affected rollout gets a `codex-history-boundary-unverified` diagnostic and a
coverage gap, and every other request still reports.

- **Root:** a paginated rollout with one request, a 6,400-token cumulative total.
- **First child:** `subagent_history_start_ordinal: 3` with a copied prefix that holds
  no `token_count`, then two own counters.
  The first reports a 9,100-token total with a 2,700-token last usage, so the inherited
  6,400 tokens cannot be proven and the step is excluded.
  The second step’s delta from 9,100, 1,600 tokens, equals its last usage and counts.
- **Second child:** `subagent_history_start_ordinal` is the string `"3"`, which places
  none of the rollout’s usage, so its copied 6,400-token counter and its own 1,050
  tokens are both excluded.
- **Reconciled:** 2 requests and 8,000 tokens, with partial coverage: one gap per child
  and one diagnostic per child, 3 excluded records in all.
- **Naive sum:** each rollout’s last cumulative total gives 24,550 tokens.
- **Shapes:** the first child is the repository’s `paginated-subagent` shape as a
  counter-only writer would produce it, as review A of PR #16 described.
  The string boundary is synthetic and stands for damaged or foreign-written metadata.
