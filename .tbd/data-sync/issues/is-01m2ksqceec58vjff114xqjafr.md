---
type: is
id: is-01m2ksqceec58vjff114xqjafr
title: Extend the transcript sanitizer for the cases it cannot cover
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
  - fixtures
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:26:33.804Z
updated_at: 2026-09-16T00:26:33.804Z
---
scripts/sanitize-claude-fixture.mjs (with tests) turns Claude Code transcript excerpts into structure-only fixtures. Limits found while deriving claude-project/derived-block-records-subagent:

- It keeps enum-like values under an allowlist of keys (type, role, model, version, agentType, status, rateLimitType and others). A workspace-specific value, such as a custom agentType or mode, would survive, so every output still needs review; consider an opt-in strict mode that replaces unknown enum values too.
- It handles claude-project only. Codex rollouts (thread and rollout IDs in file names, ordinals, .jsonl.zst members) and captured streams need their own path before a real Codex or claude-stream case can be derived.
- It writes records only: expected.json and the case README are still written by hand, which is where the reconciled truth comes from.
- Excerpt selection is manual. A small helper that cuts a window around a chosen response or spawn would make later cases cheaper.
