# facets

A synthetic Cursor state snapshot with five composers, each a different purpose and
catalog family:

- `agent` / Claude / `style-2` (provider anthropic)
- `chat` / Gemini / `style-1` (provider google)
- `plan` / GPT (provider openai)
- `multitask` / Composer (provider cursor)
- `background` / Kimi (provider moonshot)

Every composer has nonzero bubble tokens, so naive and reconciled request counts match.
The case exists to lock `--group-by provider,agent,model,effort,purpose` on Cursor
without collapsing vendors into the agent token.
Composer `modelConfig.maxMode` is current picker state and is not recorded as historical
usage.

- **Shapes:** `urollup-cursor-state/v1` mirrors `composerData` / `bubbleId` fields from
  [research-2026-09-19-cursor-agent-logs.md](../../../../../../docs/project/research/research-2026-09-19-cursor-agent-logs.md).
- **Naive sum:** counting every composer as a request equals the reconciled 5.
