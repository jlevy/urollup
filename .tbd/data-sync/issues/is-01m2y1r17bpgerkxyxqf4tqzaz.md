---
type: is
id: is-01m2y1r17bpgerkxyxqf4tqzaz
title: Lock Cursor format facts from the research brief
kind: task
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y1r66p1rgzn4qqcyt61krw
  - type: blocks
    target: is-01m2y1r6v6fbtm50s568vskdfm
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
created_at: 2026-09-19T23:59:08.007Z
updated_at: 2026-09-20T00:05:59.286Z
---
Phase 0 of plan-2026-09-19-cursor-dialect.md. Accept docs/project/research/research-2026-09-19-cursor-agent-logs.md (uro-890b, closed) and lock dialect stores, discovery, usage-bearing records, identity keys, and fixture Cursor version.

Locked: state.vscdb composerData/bubbleId is authoritative (~4.2 GiB + WAL). JSONL is a thin export (76 of 1,022). ~/.cursor/chats and store.db absent here. Usage is partial (costInCents sometimes; bubble tokenCount often zero; no cache). Model is catalog/native id; provider is inferred; Auto is default and hides the served model. Default urollup discovery finds none of this. Risks: unofficial schema, WAL/huge DB, JSONL-only miss, UUID join double-count.

Privacy: field names and structural paths only. Spec Phase 0 checklist is checked. No adapter work starts before this bead closes.

## Notes

Research brief uro-890b is in the tree and accepted. The plan spec now cites it and locks Phase 0 to those facts (stores, usage gap, model/provider mapping, opt-in discovery, Cursor 3.21.13). Checklist in the spec is checked. Remaining open questions (Best-of-N billing, --current signal, store.db on another install, dialect token name, later Cursor versions) are adapter-time, not format-lock blockers. Ready to close after review; left open per this pass (docs only, no commit).
