---
type: is
id: is-01m2f0tewzgw6zd3vzrp2c865s
title: Implement claude-project and codex-rollout adapters
kind: task
status: closed
priority: 2
version: 15
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2f0tf88qgvz2kjm2dx4kvr1
  - type: blocks
    target: is-01m2f0tjd5bfn52eg9sztbzh7x
  - type: blocks
    target: is-01m2gbe7t7shfxqxrgfmdmz7yc
  - type: blocks
    target: is-01m2ksp8dcjmgy7awtpqjjz22v
  - type: blocks
    target: is-01m2kswb5katd7wnv9sevd37pw
  - type: blocks
    target: is-01m2kt252w8nytk407sj8p30ta
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-14T03:54:22.494Z
updated_at: 2026-09-16T05:17:02.698Z
closed_at: 2026-09-16T05:17:02.694Z
close_reason: Implemented Claude project and Codex rollout adapters with default and override discovery, manifests, native identities and hierarchy evidence, copy/revision reconciliation, per-model advisor usage, rate-limit observations, stable diagnostics, and hermetic coverage over all 28 frozen fixture cases. Added failure coverage for unreadable roots and malformed sidecars plus zero-token Claude sessions. Full make check passes; CLI transcript goldens and result checks remain ratcheted to downstream uro-d135 and uro-xsfj once commands exist.
resolution: null
duplicate_of: null
---
Milestone 0.1: claude-project and codex-rollout adapters with default discovery (CLAUDE_CONFIG_DIR, ~/.claude/projects, XDG_CONFIG_HOME/claude, subagents/ and subagents/workflows/; CODEX_HOME sessions/ and archived_sessions/, .jsonl and .jsonl.zst), UROLLUP_* override variables, snapshot manifests, source links and provider limit observations, following design §2.1, §2.2, §3.4 and §4.4; document unsupported fields and the agent versions each fixture covers. Captured-stream dialects (claude-stream, codex-exec) and the metaproc port are milestone 0.5 (uro-i6o2).

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Claude: key provider + message.id + requestId; select one whole block record per request (largest output_tokens, then last in file, then lowest src- ID) with a diagnostic when fields differ; progress-nested subagent messages, /btw replays and same-uuid fork records are parent-owned copies; advisor iterations are separate model usage in one request; keep the last iterations element as a context-size measure; read promptId, turn_duration, server_tool_use, speed, stop_reason refusal and entry effort.
- claude-stream: rate_limit_event limit records (no timestamp, native units), inline subagents by parent_tool_use_id, result usage vs modelUsage, cumulative total_cost_usd across results; captures merge with transcripts by session_id and message.id.
- Codex: token_usage_record keyed by response_id (rust-v0.153.0+); older files use cumulative token_count epochs; info:null is a limit observation; compaction estimates and context-window-full fills are diagnostics; copies by thread_id; child boundary precedence; exec captures link to rollouts by thread.started.thread_id with turn.failed and interrupted turns as coverage gaps; .jsonl.zst and gzip input; limit_id, plan_type and credits on limit observations; requested model basis; service tier only from thread_settings_applied; sources keyed by thread and rollout ID.
- Decoding: prefilters are hints; never drop a record over a nested null; accept any RFC 3339 precision; count malformed lines per source; replay sniffed lines after dialect detection; follow symlinks only within roots and report skipped links.
- Port with notices: ccusage ordered parallel file reader and line prefilter (MIT); Codex usage, session-meta, rate-limit, exec-event and hook-input types, rollout file-name parser and legacy normalizers (Apache-2.0). Spike: prefiltered borrowed typed decoding is ~2.3x faster than serde_json::Value.

Must include end-to-end goldens and result checks on the fixture cases (uro-3ht1, uro-xsfj): each case's discovery root is read through its native variable in a hermetic environment, and its reconciled totals, ownership counts, excluded copies, limit observations and diagnostics are checked against expected.json by `make e2e-results` (tests/golden/README.md).
