---
type: is
id: is-01m2f0tewzgw6zd3vzrp2c865s
title: Implement Claude and Codex adapters
kind: task
status: open
priority: 2
version: 6
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tf88qgvz2kjm2dx4kvr1
  - type: blocks
    target: is-01m2f0tjd5bfn52eg9sztbzh7x
  - type: blocks
    target: is-01m2gbe7t7shfxqxrgfmdmz7yc
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:22.494Z
updated_at: 2026-09-14T19:58:23.224Z
---
claude-project, claude-stream, codex-rollout and codex-exec adapters with default discovery, UROLLUP_* override variables, snapshot manifests, source links and provider limit observations (Codex rate_limits, Claude quotaLimits) kept verbatim; document unsupported fields and fixture agent versions. Follow the capture-first principle: keep native fields beside normalized ones.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Claude: key provider + message.id + requestId; select one whole block record per request (largest output_tokens, then last in file, then lowest src- ID) with a diagnostic when fields differ; progress-nested subagent messages, /btw replays and same-uuid fork records are parent-owned copies; advisor iterations are separate model usage in one request; keep the last iterations element as a context-size measure; read promptId, turn_duration, server_tool_use, speed, stop_reason refusal and entry effort.
- claude-stream: rate_limit_event limit records (no timestamp, native units), inline subagents by parent_tool_use_id, result usage vs modelUsage, cumulative total_cost_usd across results; captures merge with transcripts by session_id and message.id.
- Codex: token_usage_record keyed by response_id (rust-v0.153.0+); older files use cumulative token_count epochs; info:null is a limit observation; compaction estimates and context-window-full fills are diagnostics; copies by thread_id; child boundary precedence; exec captures link to rollouts by thread.started.thread_id with turn.failed and interrupted turns as coverage gaps; .jsonl.zst and gzip input; limit_id, plan_type and credits on limit observations; requested model basis; service tier only from thread_settings_applied; sources keyed by thread and rollout ID.
- Decoding: prefilters are hints; never drop a record over a nested null; accept any RFC 3339 precision; count malformed lines per source; replay sniffed lines after dialect detection; follow symlinks only within roots and report skipped links.
- Port with notices: ccusage ordered parallel file reader and line prefilter (MIT); Codex usage, session-meta, rate-limit, exec-event and hook-input types, rollout file-name parser and legacy normalizers (Apache-2.0). Spike: prefiltered borrowed typed decoding is ~2.3x faster than serde_json::Value.
