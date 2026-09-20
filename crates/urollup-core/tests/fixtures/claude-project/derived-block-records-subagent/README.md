# derived-block-records-subagent

The one case derived from real traffic: a slice of the maintainer’s own urollup session,
run through `scripts/sanitize-claude-fixture.mjs` on 2026-09-15, which keeps record
order, keys, models, `usage` numbers and links, and replaces every ID, path, name,
branch and free text with placeholders and shifts timestamps by a constant offset.
No original content is retained.
The records carry Claude Code `version` 2.1.270.

- **Layout:** a session turn with its `attachment` and `file-history-snapshot`
  neighbours, one response written as three block records (`apiBlockIndex` 0, 1, 2)
  ending in an `Agent` tool call, the session-level records that follow it, and the tool
  result. The spawned subagent’s transcript holds its opening turn, ten `attachment`
  records and its first response as two block records, beside the `.meta.json` whose
  `toolUseId` names the spawning call.
- **Reconciled:** 2 requests.
  The parent’s three block records agree on every usage field, so the last one is
  selected with no diagnostic.
  The subagent’s two disagree on `output_tokens` (10 then 183), and the earlier one
  omits `output_tokens_details`, `server_tool_use`, `iterations` and `speed` altogether,
  so the rule’s “select one whole record” matters: taking the largest `output_tokens`
  keeps 183 output and 18 reasoning tokens, while a field-wise merge would invent a
  record that never existed.
- **Naive sum:** counting every assistant record gives 5 requests, 12,847 output and
  2,774,561 cache-read tokens against 2, 4,401 and 936,920.
- **What it confirms:** block records of one response carry `apiBlockIndex`; reasoning
  arrives as `output_tokens_details.thinking_tokens`; `cache_creation` splits 5-minute
  and 1-hour writes and agreed with the flat count in this sample; `effort` sits at the
  record level; a tool result names its assistant record through
  `sourceToolAssistantUUID`; and a session file carries many non-message record types an
  adapter must skip.

Deriving more cases is the same three steps: copy the records a case needs into an
excerpt tree that mirrors the config directory, run the sanitizer into a new case
directory, then write `expected.json` and this README by hand.
The sanitizer refuses to write when its output still trips the fixture privacy check.
