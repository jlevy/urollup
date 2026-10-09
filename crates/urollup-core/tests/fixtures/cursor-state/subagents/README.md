# subagents

A synthetic Cursor state snapshot with one parent composer and two
`subagentComposerIds`:

- Parent agent composer with Grok tokens (provider cursor).
- Child subagent with Opus tokens and `thinkingStyle` 2 (provider anthropic).
- Child subagent with Gemini tokens (provider google).

Each child names the parent on `parentComposerId`. `--session` on the parent uses
descendant scope by default, so the three composers are one family, matching live Cursor
composers that spawn subagents.

- **Shapes:** `urollup-cursor-state/v1` mirrors `composerData` / `bubbleId` fields from
  [research-2026-09-19-cursor-agent-logs.md](../../../../../../docs/project/research/research-2026-09-19-cursor-agent-logs.md).
- **Naive sum:** counting every composer as a request equals the reconciled 3.
