# brief-double-counting

Reproduces the Claude half of the research brief’s
[synthetic double-counting example](../../../../../../docs/project/research/research-2026-09-13-portable-agent-usage.md#synthetic-double-counting-example)
exactly, so tests can show the 4x overcount.

- **Layout:** session `00000000-0000-4000-8000-000100000001` writes one response as
  thinking, text and tool-use block records sharing `message.id` and `requestId`, with
  `output_tokens` 12, 12 and 600. Session `00000000-0000-4000-8000-000100000002` is a
  resumed file whose only record replays the last block with its original `uuid` and
  `sessionId`.
- **Reconciled:** one request owned by the first session, using the record with the
  largest `output_tokens` (design §3.4), with all four records as evidence (§3.3). The
  copied `sessionId` is fork evidence (§3.2), so the second session owns nothing.
- **Naive sum:** 4 requests, 12 input, 160,000 cache read and 1,224 output tokens,
  against 3, 40,000 and 600 reconciled.
- **Shapes:** block records from Anthropic’s
  [session-report notes](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L14-L28);
  transcript keys from
  [agentfdr’s parser tests](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/parser.test.js).
