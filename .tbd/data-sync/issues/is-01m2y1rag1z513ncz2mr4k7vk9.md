---
type: is
id: is-01m2y1rag1z513ncz2mr4k7vk9
title: Implement Cursor discovery and usage adapter
kind: task
status: open
priority: 2
version: 10
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: unknown@spud10
labels:
  - post-0.1
dependencies:
  - type: blocks
    target: is-01m2y1redkntka5x0rwr6kjmkb
  - type: blocks
    target: is-01m2y1rhynxj8zrnp4368g534e
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-09-19T23:59:17.502Z
updated_at: 2026-10-11T06:29:18.105Z
started_at: 2026-09-20T07:01:46.146Z
---
Phase 1 adapter from plan-2026-09-19-cursor-dialect.md. Default discovery (transcripts and application user state), UROLLUP_CURSOR_DIRS, snapshot manifests, strip policies, identities, copy/resume/subagent/Best-of-N rules, and coverage gaps. Count only recorded usage fields.

Named Cursor state-store reader is in scope; generic SQLite input (Decision 20) is not. Do not invent record shapes; use the locked brief. Do not change ingest-capacity or scalable-ingestion code.

## Notes

Added workspaceId as project facet and --current/--session composer-family ingest so bubble reads skip the rest of state.vscdb. Best-of-N stays Spawn. JSONL still rejected as a usage owner. Still uncommitted.


The parent of this bead is:
---
type: is
id: is-01m2y1qw9mgfwsbpdgam8nn13r
title: "Spec: Cursor dialect with model and provider facets"
kind: epic
status: open
priority: P2
version: 12
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels: []
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
created_at: 2026-09-19T23:59:02.962Z
updated_at: 2026-09-20T07:01:46.388Z
---
Add Cursor as a urollup agent with first-class model and provider facets under docs/project/specs/active/plan-2026-09-19-cursor-dialect.md. Implementation belongs no earlier than the product Phase 3 database-input work under confirmed Decision 20, with uro-3fbc also complete. An earlier database adapter requires an explicit confirmed exception; it is outside milestone 0.1 and Phase 2 Pi/Gemini.

The existing single-installation research identifies state.vscdb composerData/bubbleId as its useful store and JSONL as a thin export. These local storage observations were not independently re-surveyed during the PR audit. The stated JSONL coverage aggregates are unreconciled and must not supply an exact missing-session total. Usage is partial: local token/cache evidence does not support Claude-grade reconciliation. Discovery remains opt-in.

Facet contract: agent=cursor. A usageData key labels that session/model cost only; a bubble modelInfo.modelName labels that bubble usage. Current selectedModels and picker state remain selection metadata, never a fallback for historical usage. Unattributed historical usage and its provider remain unknown. Provider inference uses a versioned mapping from the historically attributed model, never an invented stored column. --group-by provider is required.

Cursor current-session detection and its unsupported-dialect diagnostic are planned, not implemented. Establish an exact signal before adding either. PR14 review fixes c8befd9 and beads uro-g7da/uro-6unm/uro-oxnw/uro-1vfu govern these corrections; uro-knnz retains implementation-dependent link and integration validation.


Reconciled the plan spec with research-2026-09-19-cursor-agent-logs.md (uro-890b, closed). Phase 0 facts are locked in the spec. Goals downgraded: no Claude-grade per-request tokens/cache; discovery is opt-in (not default); model is catalog/native id, not a served provider API id; provider is inferred (including cursor for Grok/Composer). Product-plan G2 now says opted-in Cursor sessions and notes the usage gap. Design one-liners did not contradict. No adapter. No ingest-capacity Rust. uro-3fbc remains the Phase 0 gate (checklist items are checked in the spec).


2026-10-09: the uncommitted Cursor implementation from the cursor-dialect worktree is preserved as draft PR https://github.com/jlevy/urollup/pull/20 (branch cursor-state-adapter, commit 8f75b59), stacked on the plan PR #14 as formal stack #21. Fixtures are synthetic. rusqlite (bundled) still needs the Decision 20 exception (uro-2hck) and a supply-chain record.
