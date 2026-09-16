---
type: is
id: is-01m2ksqdqtvwncabnag6zkehd0
title: Add captured-stream and migrated-rollout fixture cases
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
  - fixtures
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:26:35.105Z
updated_at: 2026-09-16T00:26:35.105Z
---
Dialect cases the milestone 0.1 corpus leaves out, listed in the fixtures README's 'Not modeled yet':

- claude-stream: rate_limit_event limit records without timestamps, inline subagent messages attributed by parent_tool_use_id, result usage against modelUsage, and total_cost_usd cumulative across several result records.
- codex-exec: turn.completed.usage as a cumulative thread total across a resume, with no total_tokens, excluding subagents, and a capture beside the rollout that owns the usage (thread.started.thread_id equal to session_meta.id), plus failed and interrupted turns as coverage gaps.
- Migrated Codex rollouts: codex migrate-rollouts --apply rewrites a legacy file in place as paginated and drops rolled-back usage records, so the case needs a before and after pair and capture-store expectations (uro-xm48, uro-gnqj).

Each case follows the frozen format: a discovery root plus expected.json with reconciled truth, diagnostics and naive sums.
