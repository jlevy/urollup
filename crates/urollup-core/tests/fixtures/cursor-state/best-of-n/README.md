# best-of-n

A synthetic Cursor state snapshot with one parent composer and two Best-of-N siblings:

- Parent agent composer with Grok tokens (provider cursor).
- Sibling marked `isBestOfNSubcomposer` with Opus tokens (provider anthropic).
- Sibling marked `isBestOfNSubcomposer` with Gemini tokens (provider google).

Each sibling names the parent on `parentComposerId`. The parent lists both IDs on
`subComposerIds`. The edge is Spawn, so `--session` on the parent uses descendant scope
and counts independently recorded sibling usage.
`Other("best-of-n")` would drop those children because it does not define descendants.

- **Shapes:** `urollup-cursor-state/v1` mirrors `composerData` / `bubbleId` fields from
  [research-2026-09-19-cursor-agent-logs.md](../../../../../../docs/project/research/research-2026-09-19-cursor-agent-logs.md).
- **Naive sum:** counting every composer as a request equals the reconciled 3.
