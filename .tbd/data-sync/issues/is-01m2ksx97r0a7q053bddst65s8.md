---
type: is
id: is-01m2ksx97r0a7q053bddst65s8
title: Implement the claude-stream and codex-exec captured-stream adapters
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies:
  - type: blocks
    target: is-01m2ksd74mkre0zjh8rka36a4p
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:29:47.126Z
updated_at: 2026-09-16T00:31:53.599Z
---
Milestone 0.5: read harness-captured agent streams through urollup's own adapters, with no metaproc dependency. Design §2.1, §2.6, §3.4 and Decision 4.

Acceptance:
- claude-stream parses saved `claude -p --output-format stream-json` output: rate_limit_event limit records without timestamps in native units, inline subagents by parent_tool_use_id, result usage versus modelUsage, and cumulative total_cost_usd across results. A capture merges with its transcript by session_id and message.id (response ID).
- codex-exec parses saved `codex exec --json` output: a capture whose thread.started.thread_id names a discovered rollout is a capture of that thread, the rollout owns the usage, and the capture's cumulative turn.completed.usage totals are a reconciliation check; turn.failed and interrupted turns are coverage gaps.
- Captured streams have no standard location and enter only through --source or a manifest, with the dialect detected from the first records or given by a hint.
- metaproc run directories are discovered by content under --source: captured streams under .logs/tasks/ and, from metaproc 32cde09, preserved native logs under .logs/native/ (<stem>.codex-sessions/ rollouts and <stem>.claude-projects/ transcripts). A preserved rollout owns the usage that its codex-exec capture only checks.
- Goldens prove a run directory holding both layers counts each request exactly once, and that an older metaproc run with captures only still reports its usage.
