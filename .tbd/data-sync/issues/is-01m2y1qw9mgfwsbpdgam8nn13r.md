---
type: is
id: is-01m2y1qw9mgfwsbpdgam8nn13r
title: "Spec: Cursor dialect with model and provider facets"
kind: epic
status: open
priority: 2
version: 19
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels:
  - post-0.1
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2y1r17bpgerkxyxqf4tqzaz
  - is-01m2y1r66p1rgzn4qqcyt61krw
  - is-01m2y1r6v6fbtm50s568vskdfm
  - is-01m2y1rag1z513ncz2mr4k7vk9
  - is-01m2y1redkntka5x0rwr6kjmkb
  - is-01m2y1rhynxj8zrnp4368g534e
  - is-01m2y1rrq5qkqz1vd10tb4qcjr
  - is-01m2y50zntpg0bavafh8547sfb
  - is-01m2ysxx8r7vgcrsx7fcs99fph
  - is-01m4fsy58gnxeg87emjv1fpyf1
  - is-01m4fsy5m4zq4n83g3rz2ra615
  - is-01m4fsy5zxhtfskdjgn686rb1b
  - is-01m4fsy6bqyv66kcg2sbnvgbhj
  - is-01m4fsy97zzenmdxxbx98en2zr
created_at: 2026-09-19T23:59:02.962Z
updated_at: 2026-10-11T06:29:19.735Z
---
Add Cursor as a urollup agent with first-class model and provider facets under docs/project/specs/active/plan-2026-09-19-cursor-dialect.md. Implementation belongs no earlier than the product Phase 3 database-input work under confirmed Decision 20, with uro-3fbc also complete. An earlier database adapter requires an explicit confirmed exception; it is outside milestone 0.1 and Phase 2 Pi/Gemini.

The existing single-installation research identifies state.vscdb composerData/bubbleId as its useful store and JSONL as a thin export. These local storage observations were not independently re-surveyed during the PR audit. The stated JSONL coverage aggregates are unreconciled and must not supply an exact missing-session total. Usage is partial: local token/cache evidence does not support Claude-grade reconciliation. Discovery remains opt-in.

Facet contract: agent=cursor. A usageData key labels that session/model cost only; a bubble modelInfo.modelName labels that bubble usage. Current selectedModels and picker state remain selection metadata, never a fallback for historical usage. Unattributed historical usage and its provider remain unknown. Provider inference uses a versioned mapping from the historically attributed model, never an invented stored column. --group-by provider is required.

Cursor current-session detection and its unsupported-dialect diagnostic are planned, not implemented. Establish an exact signal before adding either. PR14 review fixes c8befd9 and beads uro-g7da/uro-6unm/uro-oxnw/uro-1vfu govern these corrections; uro-knnz retains implementation-dependent link and integration validation.

## Notes

2026-10-10: Cursor support is deferred past the initial 0.1 release by maintainer decision; 0.1 ships Claude Code and Codex only. PR #14 (plan, branch cursor-dialect at 2d4134a) and PR #20 (adapter, branch cursor-state-adapter at 2ecf386) are closed and their branches kept for later work. PR #14 review B found the plan merge-ready apart from B3 (fold 4be4ad4 into 3d1c63f before landing) and B4-B6 wording. PR #20 reviews A and B found blockers listed in uro-wf20-style follow-up (see the child bead for the #20 review findings). Maintainer decisions 2026-10-10: Cursor confirmed as a planned supported agent (uro-jfaw); SQLite dependency deferred (uro-kddq); a bubble without a per-bubble model reports an unknown model; --agent stays a 0.5 flag (uro-feyo).
