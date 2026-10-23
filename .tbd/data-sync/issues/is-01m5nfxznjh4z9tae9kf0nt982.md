---
type: is
id: is-01m5nfxznjh4z9tae9kf0nt982
title: Keep the latest Codex turn context for counters after a context without turn_id
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - correctness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:58.673Z
updated_at: 2026-10-23T23:00:58.673Z
---
Turn contexts are stored only by ID (codex_rollout.rs:1990) and current_turn = *turn_id (2003) resets to None, so every counter after a pre-0.100 turn_context without turn_id reports model and effort unknown; token totals are unaffected. docs/formats/codex.md says a counter takes the latest turn_context with a turn_id. Fix: keep the latest context for attribution even without an ID, and align the format doc.
