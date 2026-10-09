# ambiguous-owner

Tests design §4.2’s ambiguous ownership status: one response that two sessions both
prove they own is counted once, owned by neither session, and kept with the agent its
candidates share.

- **Layout:** session 1 holds response `msg_01Synthetic17A` and then
  `msg_01Synthetic17Shared` (`req_011Synthetic17Shared`). Session 2 holds the same
  shared response, with the same model, timestamp and usage but its own `sessionId` and
  fresh `uuid`s, then its own response `msg_01Synthetic17B`. Nothing links the sessions:
  no `uuid` replay, no foreign `sessionId`, no subagent.
- **Reconciled:** 3 requests and 4,035 tokens.
  The shared response is one request with ambiguous ownership, candidates both sessions,
  and a `conflicting-owners` diagnostic; each session owns only its own response.
- **Naive sums:** every assistant record gives 4 requests and counts the shared response
  twice.
- **Agent attribution:** `sessions` shows the shared request in Claude Code’s unowned
  row, with no session or project.
  `codex-rollout/ambiguous-owner` is its Codex twin, and the mixed-agent golden
  `tests/golden/mixed-agent-unowned.tryscript.md` reads both roots at once.
- **Shapes:** the record shapes of `gateway-message-id-reuse`; the ambiguity is
  synthetic, since no Claude Code release is known to write it.
