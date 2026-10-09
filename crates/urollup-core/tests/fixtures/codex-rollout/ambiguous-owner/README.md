# ambiguous-owner

Tests design §4.2’s ambiguous ownership status with its own example: a response ID that
appears in two threads with no recorded fork or resume edge is counted once, owned by
neither thread, and kept with the agent its candidates share.

- **Layout:** thread 1 records responses `resp_synthetic_c17_a1` and
  `resp_synthetic_c17_shared` in one turn.
  Thread 2, which starts while thread 1’s turn runs and has no `forked_from_id`, records
  `resp_synthetic_c17_shared` with the same model, timestamp and usage under its own
  `thread_id`, then `resp_synthetic_c17_b2` in a second turn.
- **Reconciled:** 3 requests and 26,300 tokens.
  The shared response is one request with ambiguous ownership, candidates both threads,
  and a `conflicting-owners` diagnostic; each thread owns only its own response.
- **Naive sums:** every `token_usage_record` line, or every `token_count` event’s
  `last_token_usage`, gives 4 requests and 38,900 tokens, counting the shared response
  twice.
- **Agent attribution:** `sessions` shows the shared request in Codex’s unowned row,
  with no session or project.
  `claude-project/ambiguous-owner` is its Claude Code twin, and the mixed-agent golden
  `tests/golden/mixed-agent-unowned.tryscript.md` reads both roots at once.
- **Shapes:** the record shapes of `token-usage-records`; the ambiguity is synthetic,
  since no Codex release is known to write it.
