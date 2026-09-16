# fork-subagent-uuid-replay

Tests the design §3.4 rule that a fork-style subagent record carrying a parent record’s
`uuid` is a copy owned by the parent, even when its `requestId` differs.

- **Layout:** the parent session has responses `A` and `B`; `B` spawns a fork.
  The fork transcript (no `.meta.json`) replays the parent’s first four entries with
  identical `uuid`s, marked `isSidechain` with an `agentId`. Its copy of `B` (line 4)
  carries a new `requestId`. Then it runs its own request `F`.
- **Reconciled:** 3 requests; `A` and `B` are owned by the session and `F` by the fork.
  Fork lines 2 and 4 are copies and add nothing.
- **Naive sums:** every record gives 5 requests; keying on `(message.id, requestId)`
  still counts the line 4 copy, giving 4.
- **Shapes:** Anthropic’s
  [session-report analyzer](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L541-L554)
  documents uuid replays and resolves an unlabeled subagent’s type to `fork`
  ([lines 111–156](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L111-L156)).
