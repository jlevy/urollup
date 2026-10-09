# basic

A synthetic Cursor state snapshot with three composers:

- An agent composer whose assistant bubble records Opus tokens and `thinkingStyle` 2.
- An agent composer whose assistant bubble records Grok tokens and no thinking style.
- A chat composer that stores Auto `usageData.costInCents` and a zero-token bubble.

Only the two nonzero `tokenCount` bubbles become requests.
The Auto cost is not treated as tokens.
Provider is inferred later from the catalog family, not stored on disk.
The sibling JSONL transcript uses the Opus composer UUID and is not a second usage
source.

- **Shapes:** `urollup-cursor-state/v1` mirrors `composerData` / `bubbleId` fields from
  [research-2026-09-19-cursor-agent-logs.md](../../../../../../docs/project/research/research-2026-09-19-cursor-agent-logs.md).
- **Naive sum:** counting every composer as a request includes the cost-only chat.
