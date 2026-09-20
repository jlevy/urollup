---
type: is
id: is-01m2y1qw9mgfwsbpdgam8nn13r
title: "Spec: Cursor dialect with model and provider facets"
kind: epic
status: open
priority: 2
version: 10
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
created_at: 2026-09-19T23:59:02.962Z
updated_at: 2026-09-20T00:58:20.841Z
---
Add Cursor as a urollup agent with first-class model and provider facets. Dedicated plan: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md. Not milestone 0.1, not Phase 2 Pi/Gemini, not under uro-n1cp.

Phase 0 facts are locked from research-2026-09-19-cursor-agent-logs.md (uro-890b). Authoritative store is state.vscdb (composerData/bubbleId). JSONL is a thin export. Usage is partial (sometimes costInCents; tokenCount often zero; no cache). Token-level reconciliation is not supportable from local files. Discovery is opt-in; default urollup finds none of this.

Facet contract: agent=cursor; model=catalog/native id (selectedModels.modelId else modelName); provider=inferred vendor (Grok/Composer→cursor, claude-*→anthropic, …), never a stored column. --group-by provider is required.

Parented under the product epic uro-2pp9 as later planned work.

## Notes

Reconciled the plan spec with research-2026-09-19-cursor-agent-logs.md (uro-890b, closed). Phase 0 facts are locked in the spec. Goals downgraded: no Claude-grade per-request tokens/cache; discovery is opt-in (not default); model is catalog/native id, not a served provider API id; provider is inferred (including cursor for Grok/Composer). Product-plan G2 now says opted-in Cursor sessions and notes the usage gap. Design one-liners did not contradict. No adapter. No ingest-capacity Rust. uro-3fbc remains the Phase 0 gate (checklist items are checked in the spec).
