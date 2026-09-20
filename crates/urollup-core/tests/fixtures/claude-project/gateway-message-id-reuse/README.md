# gateway-message-id-reuse

Tests design §3.6’s rule for conflicting shared keys: observations sharing a key but
disagreeing on revision-invariant fields are ambiguous and never merged (a Candidate
decision, §9.1), while a true copy still collapses.

- **Layout:** sessions 1 and 2 each hold a response with `message.id` `msg_gw_000001`
  and no `requestId`, but with different models, timestamps and content.
  Session 3 copies session 1’s two records with their original `sessionId` and `uuid`,
  then adds its own response `msg_gw_000002`.
- **Reconciled:** 3 requests.
  The two `msg_gw_000001` observations stay separate with an ambiguous identity basis
  and an `identity-key-conflict` diagnostic, each owned by its session.
  Session 3’s copy is evidence for session 1’s request, and session 3 owns only its own
  response.
- **Naive sums:** every record gives 4 requests; deduplicating by `message.id` alone
  (ccusage 20.0.20) merges the distinct responses and reports 2.
- **Shapes:** modeled on ccusage commit
  [a4b8420](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84)
  (#1661), which scopes message dedupe by session.
