---
type: is
id: is-01m2y1qw9mgfwsbpdgam8nn13r
title: "Spec: Cursor dialect with model and provider facets"
kind: epic
status: open
priority: 2
version: 8
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
created_at: 2026-09-19T23:59:02.962Z
updated_at: 2026-09-19T23:59:32.067Z
---
Add Cursor as a urollup agent with first-class model and provider facets. Dedicated plan: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md. Not milestone 0.1, not Phase 2 Pi/Gemini, not under uro-n1cp. Implementation waits on locked format facts from the Cursor research brief.

Facet contract: agent=cursor (surface); model=recorded native/served identifier (never the token cursor); provider=model vendor (anthropic, openai, xai, …), never cursor. --group-by provider is required.

Parented under the product epic uro-2pp9 as later planned work.
