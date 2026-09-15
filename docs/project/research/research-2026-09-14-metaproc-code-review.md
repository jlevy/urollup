---
title: metaproc and qm Review for urollup
description: What urollup can port, reuse or learn from metaproc's agent adapters, captured-stream parsing, usage and quota handling, design documents and engineering gates, and from the qm multiplayer agent harness, reconciled with the squares review and the log dialect survey.
author: Joshua Levy (github.com/jlevy) with LLM assistance
date: 2026-09-14
status: Complete for review and reconciled with Codex and Pi source facts (uro-ly32); some claude-stream details still need verification
---
# Research: metaproc and qm Review for urollup

## Overview

[metaproc](https://github.com/jlevy/metaproc/tree/9b2e5ad51ab16f666f9478ba81b79e0988923d11)
is the maintainer’s Python and TypeScript framework for structured multi-step agent
workflows. It launches Claude Code, Codex, Pi and Gemini CLI as subprocesses, captures
their JSON output, and reports tokens, estimated cost, tool use, provider rate limits,
credential-pool state and host resources for every run.
Its design documents record the reasoning, measurements and incidents behind those
choices. Inside its ignored `attic/` directory sits a snapshot of
[qm](https://github.com/yc-software/qm/tree/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76), a
multiplayer agent harness that drives Claude Code, Codex, Pi and OpenCode for many users
at once and keeps its own per-request usage ledger.

urollup must parse the same agents’ records and tally the same quantities, so both
projects are direct evidence for its adapters, ledger and reports, and a record of
mistakes to design against.
This review (bead `uro-ankt`) identifies what urollup should port, which fixtures and
tests it can reuse, which formats it should share, and which lessons its planning
documents do not yet cover.
It reconciles its findings with the
[squares review](research-2026-09-14-squares-code-review.md) and corrects the
[portable research brief](research-2026-09-13-portable-agent-usage.md)’s log dialect
survey. The [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md) and
[design](../../urollup-design.md) are the baseline.

Three facts frame the recommendations:

- **Provenance, not licensing, is the constraint.** metaproc is published under
  AGPL-3.0-or-later
  ([`metaproc/LICENSE:1-2`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/LICENSE#L1-L2)),
  but it is the maintainer’s own repository, so its code, fixtures, tests and documents
  can be ported into MIT urollup, recording the source repository (`jlevy/metaproc`) and
  commit, per the plan’s confirmed code reuse decision.
  qm is third-party MIT code
  ([`qm/LICENSE:1-3`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/LICENSE#L1-L3)),
  so ported qm code also keeps its license notice.
  Fixtures from either still need scrubbing for privacy.
- **Captured streams only.** Neither project reads Claude Code transcripts, Codex
  rollouts or Pi session files.
  metaproc parses the output it captures, which urollup calls the `claude-stream`,
  `codex-exec` and `pi-events` dialects; qm consumes SDK and server event streams and
  stores its own rows.
  Both add evidence for streams and harnesses, and none for persistent-log parsing,
  which the squares review covers for Claude Code transcripts and Codex rollouts,
  including rollout parsing, subagent replay, `token_count` handling and overlap-safe
  time measures. Codex and Pi format claims here were checked against Codex source at
  [`6b9826e`](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340)
  (`rust-v0.154.0`) and Pi source at
  [`d981de1`](https://github.com/earendil-works/pi/tree/d981de1229ef899957bbe968bc8dcda02a21f477)
  (coding-agent 0.85.1).
- **Harnesses hide usage from log readers.** metaproc turns off Claude and Pi
  persistence and deletes per-attempt config directories, and qm turns off persistence
  for all four agents.
  Usage from either never reaches the default roots urollup scans.

## Questions to Answer

1. How do these harnesses launch agents, capture output and identify sessions, and what
   does that imply for urollup’s discovery and reconciliation?
2. What do their parsers, fixtures and documents show about captured-stream dialects,
   deduplication and edge cases?
3. How do they tally tokens, cost, time, rate limits, quotas, budgets and accounts, and
   which choices hold up?
4. What do metaproc’s design documents establish about time, memory and resource
   accounting, ledgers and performance?
5. Which formats, filesystem patterns and engineering gates should urollup adopt or
   avoid?
6. Which fixtures and tests can urollup reuse after privacy scrubbing?

## Scope

| Source | Revision | Reviewed |
| --- | --- | --- |
| metaproc code and tests | `9b2e5ad` (the working tree differed only in `.tbd/config.yml` and `uv.lock`) | Adapters, log parsing, trace extractors, usage and pricing, log rewriting (compaction), quota and credential-pool mechanisms, resource contracts, I/O utilities, devtools, workflows, fixtures |
| metaproc documents | `9b2e5ad` | `docs/*.md`, `docs/arch/*.md`, runbooks, done and active specs, `CHANGELOG.md`, `TODO.md` |
| qm | `78dd4cc` (2026-08-03), a shallow clone with 20 commits | Harness adapters, session store, usage and timing records, budgets, metrics, model catalog, token estimation, tests, `README.md`, `SECURITY.md` |

Excluded: metaproc’s process-graph scheduler, visualization and Metabrowser plugin
except where they touch logs, usage, time or resources; qm’s Slack, web UI, skills,
deployment and sandbox code except where they touch usage.
No credential-pool file, token or account identifier was read or copied, and account and
quota sections describe mechanisms only.
Record shapes come from committed fixtures and show key names and counters.
Citations use `metaproc/<path>` and `qm/<path>` with line ranges linked at the pinned
commits.

## Findings: metaproc

### Agent Launch, Capture and Log Locations

| Agent | Command metaproc builds | Persistent log | Config root | Session identity read |
| --- | --- | --- | --- | --- |
| Claude Code | `claude -p` with `--output-format stream-json --verbose --no-session-persistence`, `--model` and `--effort` ([`metaproc/src/metaproc/adapters/claude_code.py:487-572`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/claude_code.py#L487-L572)) | None by default | `CLAUDE_CONFIG_DIR` set to a per-attempt slot ([`metaproc/src/metaproc/adapters/claude_code.py:788-836`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/claude_code.py#L788-L836)) | `system` `init` `session_id` ([`metaproc/src/metaproc/trace/extractors/claude_agent.py:157-192`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/claude_agent.py#L157-L192)) |
| Codex | `codex <top-level flags> exec --json`, with `-m` and `-c model_reasoning_effort=` before `exec` ([`metaproc/src/metaproc/adapters/codex.py:223-388`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/codex.py#L223-L388)) | Rollout still written, since `--ephemeral` is not passed | `CODEX_HOME` set to `<slot>/.codex` in pool mode, else `$HOME/.codex` ([`metaproc/src/metaproc/adapters/codex.py:446-475`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/codex.py#L446-L475)) | `thread.started` `thread_id` ([`metaproc/src/metaproc/trace/extractors/codex_agent.py:104-117`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/codex_agent.py#L104-L117)) |
| Pi | `pi --mode json -p @<prompt> --no-session` ([`metaproc/src/metaproc/adapters/pi_cli.py:276`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/pi_cli.py#L276)) | None | Not scoped | `session` header `id` |

- **Captures:** each attempt’s output lands in
  `.logs/tasks/<step>/<item>/<step>_<item>_<time>.jsonl` beside a
  `<log>.invocation.json` sidecar with `captured_at`, `argv`, `cwd`, the environment
  redacted by variable name, and run metadata
  ([`metaproc/src/metaproc/runpool/backend.py:34-115`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/runpool/backend.py#L34-L115)).
  The artifact catalog places the sidecar under `.state/tasks/`
  ([`metaproc/docs/artifact-catalog.md:70-81`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/artifact-catalog.md#L70-L81)),
  but the code writes it beside the log.
- **Slots are deleted:** pool slots live under `.state/auth/<step>/<item>/a<N>/`
  ([`metaproc/src/metaproc/dispatch/slot_coordinator.py:141-146`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/slot_coordinator.py#L141-L146))
  and are removed after every attempt, keeping only files an adapter declares
  diagnostic, such as Claude’s `claude-code-debug.log`
  ([`metaproc/docs/arch/arch-authentication.md:1076-1107`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-authentication.md#L1076-L1107)).
  Any transcript or rollout written under a slot was lost.
  *(Fixed 2026-09-15.)* [metaproc#82](https://github.com/jlevy/metaproc/pull/82)
  (`32cde09`, closing [#81](https://github.com/jlevy/metaproc/issues/81)) copies pooled
  Codex rollouts, and Claude transcripts when persistence is on, to
  `<run>/.logs/native/<step>[/<item>]/<session-stem>.codex-sessions/` and
  `.claude-projects/` before teardown, never copying credential files.
- **Cloud runs:** run state and logs live on shared NFS under
  `/mnt/filestore/runs/<run_id>/`, container homes are ephemeral, and archiving is an
  operator command with no documented retention policy
  ([`metaproc/docs/arch/arch-cloud-execution.md:120-153`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-cloud-execution.md#L120-L153),
  [`metaproc/docs/arch/arch-cloud-execution.md:436-473`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-cloud-execution.md#L436-L473)).
- **Config directory limits:** `CLAUDE_CONFIG_DIR` scopes user-global state but not
  project `.claude/` directories, and several upstream bugs weaken its isolation
  ([`metaproc/docs/arch/arch-claude-code-harness.md:87-106`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-claude-code-harness.md#L87-L106)).
- **Version drift:** metaproc pins Claude Code 2.1.207, Codex 0.144.1 and Pi 0.72.1
  ([`metaproc/src/metaproc/adapters/claude_code.py:54`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/claude_code.py#L54),
  [`metaproc/src/metaproc/adapters/codex.py:49`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/codex.py#L49),
  [`metaproc/src/metaproc/adapters/pi_cli.py:40`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/pi_cli.py#L40)),
  sets `DISABLE_UPDATES=1` because versions changed mid-cohort, and keeps a behavior
  matrix across 2.1.x releases
  ([`metaproc/docs/arch/arch-claude-code-harness.md:264-332`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-claude-code-harness.md#L264-L332)).
- **Requested versus observed model:** Claude Code and Gemini CLI silently fall back to
  a default for an unknown model name, and `codex exec` streams carry no model ID, so
  metaproc verifies the model from `system.init` and takes Codex’s from `argv`
  ([`metaproc/docs/arch/arch-testing.md:88-107`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-testing.md#L88-L107)).
  Codex rollouts also record only the requested model in `turn_context.model`, and a
  server reroute is never persisted
  ([`codex-rs/core/src/session/turn.rs:2589-2601`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2589-L2601)).
- **Inherited variables:** metaproc removes only `CLAUDECODE` and
  `CLAUDE_CODE_ENTRYPOINT` from child environments
  ([`metaproc/src/metaproc/adapters/claude_code.py:623-639`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/claude_code.py#L623-L639)),
  so other agent variables reach nested agents, as the plan’s ambiguity rule for
  `--current` assumes.
  Codex narrows the ambiguity for its own tools: a tool process gets its own thread’s
  `CODEX_THREAD_ID` (a subagent’s, inside a subagent) and the root `CODEX_SESSION_ID`
  ([`codex-rs/core/src/unified_exec/process_manager.rs:1370-1377`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/unified_exec/process_manager.rs#L1370-L1377),
  [`codex-rs/core/src/exec_env.rs:40-50`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs#L40-L50)).
  Codex hooks receive no `CODEX_*` identity variables of their own, only the Codex
  process environment captured at session start, so a hook in a nested Codex sees the
  outer Codex’s values; hook input `session_id` is always the root session
  ([`codex-rs/hooks/src/registry.rs:73-82`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/registry.rs#L73-L82),
  [`codex-rs/core/src/hook_runtime.rs:151-158`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L151-L158)).

For urollup, harness-driven Claude Code and Pi usage exists only in captured streams
that default discovery never sees, and a local Codex run leaves both a `codex-exec`
capture and a `codex-rollout` file for one thread.
`codex-exec` records no response IDs, so under the contracts’ `req-` key precedence the
two files yield different request keys.
The only key they share is `thread.started.thread_id`, which equals the rollout’s
`session_meta.id`
([`codex-rs/exec/src/event_processor_with_jsonl_output.rs:396-400`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L396-L400)),
so the rollout must own the usage and the capture serve as a reconciliation check.

### Captured-Stream Dialect Evidence

metaproc’s committed captures are real agent output with strings replaced by its fixture
generator, and they show details the portable brief does not record.

**`claude-stream`**, from Claude Code 2.1.126
([`metaproc/tests/fixtures/trace_agents/claude-sample.jsonl:1-19`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/trace_agents/claude-sample.jsonl#L1-L19))
and 2.1.81
([`metaproc/tests/fixtures/log_compaction/claude_sample.jsonl:1-20`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/log_compaction/claude_sample.jsonl#L1-L20)):

- **Hooks:** with hooks configured, `system` `hook_started` and `hook_response` records
  precede `init`, and `hook_response` carries hook `stdout`, `stderr` and `output`.
- **Timestamps:** only `user` tool-result records carry `timestamp`; `system`,
  `assistant`, `rate_limit_event` and `result` records have none.
- **Content blocks:** assistant records repeat one `message.id` per content block with
  identical `usage`, including `output_tokens` of 8 on a tool-use block, and have no
  `requestId`. In transcripts, Anthropic’s receipts plugin reports about 13% of one
  response’s block records disagreeing on `output_tokens`; these short captures show no
  disagreement, so per-message output still needs checking against the result.
- **Init:** `system` `init` records `claude_code_version`, `model`, `apiKeySource`,
  `permissionMode` and `session_id`.
- **Limits:** `rate_limit_event.rate_limit_info` holds `status`, `rateLimitType`
  (`five_hour` in both captures), `resetsAt` in epoch seconds, `isUsingOverage`,
  `overageStatus`, `overageResetsAt` and `overageDisabledReason`. metaproc also reads an
  optional `utilization` fraction
  ([`metaproc/src/metaproc/dispatch/auth_usage.py:262-276`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/auth_usage.py#L262-L276)),
  which neither capture contains, and documents one record per window state change
  rather than per request
  ([`metaproc/src/metaproc/dispatch/auth_usage.py:202-222`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/auth_usage.py#L202-L222)).
- **Result:** top-level `usage` adds `iterations`, `server_tool_use`, `service_tier` and
  `speed`; `modelUsage` maps each model to tokens, `costUSD`, `contextWindow` and
  `webSearchRequests`; the record also has `total_cost_usd`, `num_turns`, `duration_ms`,
  `duration_api_ms`, `terminal_reason` and `permission_denials`. In the 2.1.126 capture,
  top-level `usage` equals the Opus `modelUsage` entry, `modelUsage` adds a Haiku entry,
  and `total_cost_usd` is the sum of both `costUSD` values.
  metaproc treats `modelUsage` as authoritative because top-level `usage` can omit
  subagent and other-model contributions
  ([`metaproc/src/metaproc/logutil/usage.py:239-301`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L239-L301),
  tested in
  [`metaproc/tests/test_usage.py:384-443`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_usage.py#L384-L443)).
  Transcripts split usage the same way: top-level `message.usage` excludes
  `advisor_message` iterations, which carry their own model
  ([`ccusage/rust/crates/ccusage/src/main.rs:320-343`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L320-L343)).
- **Errors:** failed results carry `api_error_status`, such as 401 or 429, and no usage
  ([`metaproc/tests/fixtures/claude_api_signals/api_429_rate_limit.jsonl:1-2`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/claude_api_signals/api_429_rate_limit.jsonl#L1-L2)).

**`codex-exec`**
([`metaproc/tests/fixtures/trace_agents/codex-sample.jsonl:1-11`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/trace_agents/codex-sample.jsonl#L1-L11)):

- **Usage:** `turn.completed.usage` has `input_tokens`, `cached_input_tokens`,
  `output_tokens` and `reasoning_output_tokens`; metaproc treats input as including
  cached input and reasoning as part of output
  ([`metaproc/src/metaproc/logutil/usage.py:304-327`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L304-L327)).
  Its docstring, written for 0.124.0, says the reasoning field was removed, yet this
  later capture has it, so field presence varies by release; at Codex 0.154 the event
  also has `cache_write_input_tokens` and never has `total_tokens`.
- **Cumulative totals:** despite the event name, `turn.completed.usage` is the thread’s
  cumulative running total, not the turn’s: after `codex exec resume` it includes
  earlier runs, and it excludes subagent usage
  ([`codex-rs/exec/src/event_processor_with_jsonl_output.rs:117-128`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L117-L128),
  [`codex-rs/core/src/session/mod.rs:1464-1471`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1464-L1471),
  [`codex-rs/exec/src/lib.rs:1577-1604`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/lib.rs#L1577-L1604)).
  One capture ends after its first terminal turn, so per-turn figures are differences
  between consecutive captures’ totals for the same thread.
- **Failures:** `turn.failed` carries only `error`
  ([`metaproc/tests/test_adapters_codex.py:301-306`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_adapters_codex.py#L301-L306)),
  and an interrupted turn emits no terminal event at all
  ([`codex-rs/exec/src/event_processor_with_jsonl_output.rs:531-556`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L531-L556)),
  so such a turn’s usage never reaches the capture, although responses completed before
  the failure stay in the rollout.
  Reconnect notices arrive as non-terminal `error` events
  ([`metaproc/docs/arch/arch-metaproc-core.md:1728-1737`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-metaproc-core.md#L1728-L1737)).
- **Detection:** metaproc records that Codex releases before 0.130 wrote a Claude-shaped
  stream, so it identifies the dialect from record types, never from the sidecar’s
  adapter name
  ([`metaproc/src/metaproc/trace/extractors/codex_agent.py:511-537`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/codex_agent.py#L511-L537)).
- **Model:** records name no model; metaproc falls back to `-m` in the sidecar’s `argv`
  ([`metaproc/src/metaproc/trace/extractors/codex_agent.py:462-474`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/codex_agent.py#L462-L474)).

**`pi-events`**
([`metaproc/tests/fixtures/log_compaction/pi_sample.jsonl:1-80`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/log_compaction/pi_sample.jsonl#L1-L80)
and
[`metaproc/tests/fixtures/trace_agents/pi-sample.jsonl:1-19`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/trace_agents/pi-sample.jsonl#L1-L19)):

- **Repeated usage:** the event stream repeats one assistant message’s usage across five
  event types: `message_start` as zeros, each `message_update` as a partial count (41
  updates for 8 `message_end` records), `message_end` as the only final count with
  `responseId`, and copies in `turn_end.message` and `agent_end.messages[]`
  ([`pi/packages/agent/src/agent-loop.ts:200-272`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/agent-loop.ts#L200-L272)).
  The update shape is version-bounded: before Pi 0.84.0 each update carries the
  cumulative message (the shape in these captures), 0.84.0 removed it, and 0.84.2 added
  a top-level cumulative `usage`
  ([`pi/packages/coding-agent/CHANGELOG.md:237`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L237),
  [`:387`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L387)).
  A persistent Pi session file writes each message’s usage once, at `message_end`;
  repeats there come only from fork, clone, export and import copies and from copies
  nested in context compaction entries and extension payloads.
- **Timestamps:** ISO on the `session` header and epoch milliseconds inside `message`;
  tool execution events have none
  ([`metaproc/src/metaproc/logutil/parsing.py:765-782`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L765-L782)).
- **Cost and models:** a `vertex-maas` message with 6,951 input tokens reports
  `cost.total` of 0, because Pi’s cost is an estimate from its model catalog and zero
  can mean unpriced; model IDs carry organization prefixes such as `zai-org/glm-5-maas`.
- **Size:** before Pi 0.84.0, cumulative `message_update` events make logs grow
  quadratically and embed whole files.
  metaproc drops them as it writes
  ([`metaproc/src/metaproc/engine/runtime.py:31-33`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/runtime.py#L31-L33)),
  measured 41 MB of JSON for one browser view of such a log
  ([`metaproc/docs/performance-notes.md:103-117`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L103-L117)),
  and flags logs over 100 MiB with more than 500 KiB per output token after a Vertex
  streaming anomaly measured at about 12 MB per token
  ([`metaproc/docs/arch/arch-metaproc-core.md:2159-2180`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-metaproc-core.md#L2159-L2180)).

### Harness Transforms of Captured Logs

metaproc rewrites captured logs after each attempt, a step its code calls log compaction
and this review calls a **log rewrite** to keep it distinct from context compaction
([`metaproc/docs/arch/arch-metaproc-core.md:2053-2093`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-metaproc-core.md#L2053-L2093)):

- **Rewrite header:** it prepends
  `{"type": "compaction", "version": 1, "adapter", "original_size", "original_lines", "timestamp"}`
  ([`metaproc/src/metaproc/logutil/compaction.py:236-336`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/compaction.py#L236-L336)).
- **Dropped events:** Pi `message_start`, `message_update`, `turn_start`, `turn_end` and
  `tool_execution_update`; Codex `item.started`, `item.updated` and reconnect `error`
  events
  ([`metaproc/src/metaproc/logutil/compaction.py:545-591`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/compaction.py#L545-L591)).
- **Synthetic records:** for Pi it re-emits the last `message_update` as `message_final`
  when `message_end` lacks thinking blocks; `message_final` is metaproc’s own record,
  not a Pi event, and it carries partial usage
  ([`metaproc/src/metaproc/logutil/compaction.py:353-440`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/compaction.py#L353-L440)).
- **Gzip:** it writes `<log>.jsonl.gz`, verifies it and deletes the original
  ([`metaproc/src/metaproc/logutil/compaction.py:594-630`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/compaction.py#L594-L630));
  readers treat `x.jsonl` and `x.jsonl.gz` as one logical file
  ([`metaproc/src/metaproc/io/gz_io.py:37-104`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/gz_io.py#L37-L104)).
- **Rewrite window:** the rewrite replaces the file through a backup rename, so the path
  is briefly absent, and a failure classifier that read during that window misclassified
  a 401
  ([`metaproc/docs/arch/arch-authentication.md:1119-1127`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-authentication.md#L1119-L1127)).

The committed fixtures are themselves rewritten, so they begin with this header.

### Parsing Behavior and Bugs

metaproc’s parsers serve live display and per-attempt summaries.
Several of their choices would be accounting bugs in urollup:

- **Terminal-only usage:** usage comes only from Claude `result`, Codex `turn.completed`
  and Pi `agent_end`
  ([`metaproc/src/metaproc/logutil/parsing.py:1772-1811`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L1772-L1811),
  [`metaproc/src/metaproc/trace/extractors/pi_agent.py:96-126`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/pi_agent.py#L96-L126)).
  A killed or timed-out attempt records none although its requests were billed, and the
  80-line Pi capture ends mid-message with no `agent_end`.
- **Totals read as turns:** the tailer overwrites usage at each `turn.completed`, and
  the Codex trace extractor reads only the first
  ([`metaproc/src/metaproc/trace/extractors/codex_agent.py:104`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/codex_agent.py#L104)).
  A capture has one terminal turn, so that is harmless within a file, but the value is
  the thread’s cumulative total, so an attempt that resumed a thread would be charged
  every earlier run again.
- **Partial tails:** the tailer reads to end of file, splits lines and advances past an
  unfinished line, so the fragment becomes a raw event and the remainder is lost; a
  shrinking file silently resets the offset
  ([`metaproc/src/metaproc/logutil/parsing.py:1681-1721`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L1681-L1721)).
  The incremental-read test covers neither case
  ([`metaproc/tests/test_log_parsing.py:925-953`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_log_parsing.py#L925-L953)).
- **Silent tolerance:** shared JSONL readers skip malformed lines and decode with
  `errors="replace"`
  ([`metaproc/src/metaproc/io/gz_io.py:107-135`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/gz_io.py#L107-L135)),
  and the gzip size trailer is read modulo 2^32
  ([`metaproc/src/metaproc/io/gz_io.py:137-147`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/gz_io.py#L137-L147)).
- **Lossy attribution:** an attempt’s tokens go to its dominant model
  ([`metaproc/src/metaproc/logutil/usage.py:454-476`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L454-L476)),
  a display path counts cache reads as input
  ([`metaproc/src/metaproc/logutil/parsing.py:1744-1747`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L1744-L1747)),
  and price lookup falls back to the model name after its last `/`
  ([`metaproc/src/metaproc/logutil/usage.py:129-137`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L129-L137)).
- **Detection window:** dialect detection reads 20 lines
  ([`metaproc/src/metaproc/logutil/parsing.py:1498-1574`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L1498-L1574)),
  and hook records or a rewrite header can push the distinguishing record past it.
- **Tool taxonomy:** one alias table maps Claude, Pi and Gemini tool names to a family,
  operation and role
  ([`metaproc/src/metaproc/trace/extractors/common.py:96-200`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/common.py#L96-L200)),
  and Codex `command_execution`, `file_change`, `mcp_tool_call` and `web_search` items
  map onto it
  ([`metaproc/src/metaproc/trace/extractors/codex_agent.py:357-402`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/codex_agent.py#L357-L402)).
  Codex writes one `file_change` per edit, so metaproc collapses consecutive same-path
  edits
  ([`metaproc/src/metaproc/trace/extractors/common.py:317-380`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/common.py#L317-L380));
  raw tool counts mean different things per agent.
- **Tool identity:** tool spans pair by native invocation ID before any fallback, fall
  back to an ordinal scoped to the source log and tool name, and let a terminal
  aggregate count add only the positive residual beyond unique spans
  ([`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:154-172`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L154-L172)).

Unknown values keep turning into zeros despite an explicit rule against it.
Commit `12e3cfe` fixed tool durations that summed as zero by returning `None`
([`metaproc/src/metaproc/trace/extractors/common.py:382-398`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/common.py#L382-L398)),
and the resource plan lists review defects of the same family: list estimates shown as
actual cost, cached input priced twice, nested Claude usage omitted, tool calls counted
twice and measured zeros becoming unknown
([`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:67-80`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L67-L80)).

### Usage, Cost and Pricing

Choices that hold up:

- **Disjoint token buckets:** adapters normalize to uncached input, cache reads, cache
  writes and output. Codex subtracts cached from inclusive input, Claude and Pi already
  exclude cache, and Gemini recovers unreported reasoning as the larger of visible
  output and total minus input
  ([`metaproc/src/metaproc/logutil/usage.py:304-327`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L304-L327),
  [`metaproc/src/metaproc/logutil/usage.py:377-390`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L377-L390)).
- **Cost views:** agent-reported dollars and price-table amounts are list estimates;
  actual cost enters only from an external provider-authoritative event, and turns,
  steps and retries are never request counts
  ([`metaproc/docs/arch/arch-metaproc-core.md:2369-2374`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-metaproc-core.md#L2369-L2374),
  [`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:174-200`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L174-L200)).
- **Coverage states:** provider meters are `measured`, `estimated` or `unmeasured`,
  non-measured values require lineage, and rollups take the weakest state
  ([`metaproc/src/metaproc/models/resources.py:85-183`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/models/resources.py#L85-L183)).
  `api_requests` is always unmeasured, because no captured stream proves a request
  boundary
  ([`metaproc/src/metaproc/logutil/agent_provider_meters.py:1-37`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/agent_provider_meters.py#L1-L37)).
- **Reporting-only budgets:** budgets evaluate to `within`, `near`, `exceeded` or
  `unmeasured` and never stop work; `max_budget_usd` compares against list cost, never
  actual cost
  ([`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:214-226`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L214-L226)).

Choices that do not:

- **Unknowns absorbed:** metric rollups skip `None` and sum the rest, so a parent with
  an unknown child looks complete
  ([`metaproc/src/metaproc/engine/resource_rollup.py:800-816`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/resource_rollup.py#L800-L816));
  trace aggregation reads missing values as 0.0
  ([`metaproc/src/metaproc/trace/aggregation.py:103-136`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/aggregation.py#L103-L136));
  a computed price of zero is dropped as unknown
  ([`metaproc/src/metaproc/trace/extractors/common.py:260-310`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/common.py#L260-L310));
  `usage.md` omits zeros, so zero and unknown serialize alike
  ([`metaproc/src/metaproc/models/usage.py:157-191`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/models/usage.py#L157-L191));
  the YAML writer suppresses `None` values by default
  ([`metaproc/docs/arch/arch-file-io-utilities.md:118-125`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-file-io-utilities.md#L118-L125));
  and missing evidence timestamps become 1970-01-01
  ([`metaproc/src/metaproc/logutil/resource_event_extract.py:48`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/resource_event_extract.py#L48)).
- **Floats:** rates and costs are floats, rounded to three decimals when written
  ([`metaproc/src/metaproc/models/usage.py:139-146`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/models/usage.py#L139-L146)).
- **Zero source cost:** Pi’s `cost.total` of 0 is kept as a measured zero by design
  ([`metaproc/tests/test_usage.py:806-846`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_usage.py#L806-L846)),
  although in the capture it reflects a provider Pi does not price.
- **Merge:** meter rollup merge raises on overlapping event IDs instead of deduplicating
  ([`metaproc/src/metaproc/engine/resource_reconciliation.py:134-152`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/resource_reconciliation.py#L134-L152)).

The price table
([`metaproc/src/metaproc/data/pricing.md:1-282`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/data/pricing.md#L1-L282),
[`metaproc/src/metaproc/models/pricing.py:12-58`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/models/pricing.py#L12-L58))
records per-model `actual_price`, optional `list_price`, `cost_source`, `source_url` and
`last_reviewed`, and a test requires the file date to be no older than any row’s review
date
([`metaproc/tests/test_usage.py:318-331`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_usage.py#L318-L331)).
Its notes describe cases that only effective-dated rows can reprice: a promotional rate
whose end date was extended, a cache-read rate cut mid-life, a 200K-token prompt band,
one model priced differently through Vertex and the direct API, a `[1m]` context
variant, and deleted rows for older models with history left to git
([`metaproc/src/metaproc/data/pricing.md:15`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/data/pricing.md#L15),
[`metaproc/src/metaproc/data/pricing.md:54`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/data/pricing.md#L54),
[`metaproc/src/metaproc/data/pricing.md:124`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/data/pricing.md#L124),
[`metaproc/src/metaproc/data/pricing.md:186-195`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/data/pricing.md#L186-L195)).

### Provider Limits, Quotas and Accounts

- **Limit records:** metaproc reads only `claude-stream` `rate_limit_event` records.
  It never parses Codex `rate_limits`, which also carry `limit_id`, `plan_type` and
  `credits`, or transcript `quotaLimits`, and its Codex quota methods return `None`
  ([`metaproc/src/metaproc/adapters/codex.py:803-827`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/codex.py#L803-L827)).
  It keeps only the `one_hour`, `five_hour` and `seven_day` types
  ([`metaproc/src/metaproc/dispatch/auth_usage.py:222-259`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/auth_usage.py#L222-L259))
  and treats a record as blocking only when `status` is `blocked`, although its own
  status type also lists `rejected`
  ([`metaproc/src/metaproc/dispatch/auth_usage.py:56-77`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/auth_usage.py#L56-L77),
  [`metaproc/src/metaproc/logutil/parsing.py:205-220`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L205-L220)).
- **Nondeterministic merge:** across sessions a record replaces another only when both
  have a `ts`; the real records have none, so the survivor depends on set iteration
  order
  ([`metaproc/src/metaproc/dispatch/auth_usage.py:480-536`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/auth_usage.py#L480-L536)).
- **Inferred quota:** metaproc also calls an undocumented OAuth usage endpoint with a
  live token
  ([`metaproc/src/metaproc/adapters/claude_code.py:1314-1404`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/claude_code.py#L1314-L1404))
  and parses error text such as “resets 11am (America/Los_Angeles)” into the next
  Pacific occurrence
  ([`metaproc/src/metaproc/adapters/claude_code.py:175-241`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/claude_code.py#L175-L241),
  [`metaproc/src/metaproc/agent_errors.py:26-126`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/agent_errors.py#L26-L126)).
  Neither is a recorded window.
- **Units:** `rate_limit_event.utilization` is a fraction, the OAuth endpoint returns a
  fraction or a percent that metaproc rescales
  ([`metaproc/tests/test_claude_live_quota.py:56-120`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_claude_live_quota.py#L56-L120)),
  and Codex `used_percent` is a percent.
- **Account identity:** pool entries are keyed by operator-chosen labels.
  Schema version 2 adds optional `account_id`, documented as a hash over an account
  email or token (a token hash changes on rotation), `organization_uuid`, and a
  `quota_group` of kind `org`, `account` or `unknown`
  ([`metaproc/src/metaproc/dispatch/credential_pool.py:160-214`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/credential_pool.py#L160-L214)).
  Credential fingerprints rotate and identify nothing.
- **Sessions to accounts:** `auth_lease_acquired` events join
  `(run_id, step_id, item, attempt, session_log_path)` to `(adapter, label)`, and a miss
  yields `unknown` rather than a timestamp guess
  ([`metaproc/src/metaproc/runpool/event_models.py:136-195`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/runpool/event_models.py#L136-L195),
  [`metaproc/src/metaproc/dispatch/auth_usage.py:381-454`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/auth_usage.py#L381-L454)).
  Events carry labels and fingerprints, never credentials
  ([`metaproc/docs/arch/arch-authentication.md:995-1002`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-authentication.md#L995-L1002)).
- **Shared limits:** on a rate limit metaproc excludes every label in the failing
  label’s quota group, because its notes, citing a Claude Code issue, report that
  Anthropic applies limits per account and per organization
  ([`metaproc/docs/arch/arch-authentication.md:1251-1261`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-authentication.md#L1251-L1261)).
  How provider, account, model and region compose a quota namespace is left open, and
  keying by execution profile is called wrong in both directions
  ([`metaproc/docs/execution-model-design.md:228-240`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/execution-model-design.md#L228-L240)).
- **Wrong configured account:** an ambient `ANTHROPIC_AUTH_TOKEN`, `apiKeyHelper` or
  `ANTHROPIC_API_KEY` can silently override the credential a slot was given
  ([`metaproc/docs/arch/arch-claude-code-harness.md:273-275`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-claude-code-harness.md#L273-L275)).
  The stream’s observed `apiKeySource` is evidence that exposes this.
- **Limit exits look like crashes:** a rate-limit-rejected CLI exits with code 1 in 15
  to 45 seconds against 10 to 22 minutes for real work
  ([`metaproc/docs/arch/arch-authentication.md:987-993`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-authentication.md#L987-L993)),
  and prepending debug text to an error string made a 429 match a known-bug regex and
  abort 16 items
  ([`metaproc/docs/arch/arch-claude-code-harness.md:334-380`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-claude-code-harness.md#L334-L380)).

### Time Measures

- **Event logs over summaries:** a generated summary reported 13 minutes of verification
  where the event log showed 42, so metaproc treats `process-events.jsonl` and runpool
  events as the timing source of truth
  ([`metaproc/docs/performance-notes.md:39-51`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L39-L51)).
- **Timestamp discipline:** evidence with no timestamp takes a fixed sentinel rather
  than wall-clock time, and derived IDs never use file modification time
  ([`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:113-125`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L113-L125));
  missing or reversed tool timestamps stay unmeasured rather than zero or negative
  ([`metaproc/docs/releases/v0.2.1.md:26-28`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/releases/v0.2.1.md#L26-L28)).
  The sentinel is 1970-01-01, which would land in a real calendar bucket if urollup
  copied it.
- **Clock as input:** the execution-model reducer takes `now` as a parameter, so backoff
  and deadlines are testable without waiting
  ([`metaproc/docs/arch/arch-execution-model.md:44-67`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-execution-model.md#L44-L67)).
- **Tool intervals:** paired `tool_use` and `tool_result` timestamps are published as
  tool latency
  ([`metaproc/docs/releases/v0.2.1.md:26-28`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/releases/v0.2.1.md#L26-L28)),
  the same interval squares declines to call execution time.
  The interval includes scheduling and permission waits, so it is neither latency nor
  execution time; both reviews call it a **tool interval** (C2).
- **Nested sums:** `wall_time_s` is additive in the rollup and emitted at step, item and
  session levels, which nest, and `rss_bytes_max` takes a maximum across concurrent
  siblings
  ([`metaproc/src/metaproc/engine/resource_rollup.py:64-92`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/resource_rollup.py#L64-L92),
  [`metaproc/src/metaproc/engine/resource_rollup.py:800-816`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/resource_rollup.py#L800-L816)).
  The [squares review](research-2026-09-14-squares-code-review.md) defines the
  overlap-safe measures (agent-active seconds, busy union, parallel overlap and
  exclusive tool categories) that replace these sums.
- **Waiting categories:** the rollup has `wait_throttling_s`, `wait_rate_limit_s`,
  `wait_budget_s`, `wait_network_s`, `tool_exec_s` and `local_compute_s`
  ([`metaproc/src/metaproc/engine/resource_rollup.py:64-92`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/resource_rollup.py#L64-L92))
  that no document defines, and the scheduler’s blocker vocabulary defines
  `admission_wait` and `budget_wait` without ever returning them
  ([`metaproc/docs/arch/arch-execution-model.md:94-110`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-execution-model.md#L94-L110)).
- **Inferred throttling:** a blocked rate-limit record opens a window that the next
  event of any kind closes, and that gap is reported as backoff time
  ([`metaproc/src/metaproc/logutil/throttling.py:58-93`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/throttling.py#L58-L93));
  `claude-stream` limit records carry no timestamp, so the window has no recorded start.

### Memory and Resource Accounting

metaproc’s
[memory accounting reference](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/memory-accounting-reference.md)
settles, with citations to kernel sources, which counters a resource collector should
read
([`metaproc/docs/memory-accounting-reference.md:16-25`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/memory-accounting-reference.md#L16-L25)):

| Question | Misleading counter | Correct source |
| --- | --- | --- |
| Host memory free (macOS) | `kern.memorystatus_level` | `vm_stat` free, inactive and purgeable pages |
| Host memory free (Linux) | `MemFree` | `MemAvailable` |
| Process cost (macOS) | RSS | `phys_footprint` |
| Process cost (Linux) | RSS | PSS from `smaps_rollup` |
| Fan-out cost | Sum of per-process RSS | Sum of footprint or PSS over each process tree |
| Degradation | Any memory level | Stall time (Linux PSI) or paging rate |

- **Measured on macOS:** a node process read 58.4 MB RSS against a 91 MB
  `phys_footprint` and a 361 MB footprint peak, because RSS omits compressed pages and
  counts shared text once per process
  ([`metaproc/docs/memory-accounting-reference.md:85-126`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/memory-accounting-reference.md#L85-L126)).
  Lifetime swap counters need deltas
  ([`metaproc/docs/memory-accounting-reference.md:152-171`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/memory-accounting-reference.md#L152-L171)).
- **Adapter footprints:** per-process-tree RSS over three days of production batches was
  P50 384 MB and P95 748 MB for Claude Opus runs (5,714 samples) and P50 176 MB and P95
  989 MB for Pi, with Codex measured only idle; the document labels these a floor
  because RSS is the wrong metric
  ([`metaproc/docs/arch/arch-runpool.md:434-497`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-runpool.md#L434-L497)).
- **Cgroup limits:** GCP Batch’s default 2 GiB container cgroup OOM-killed agents on a
  64 GiB host
  ([`metaproc/docs/arch/arch-cloud-execution.md:405-434`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-cloud-execution.md#L405-L434)).
- **Current gaps:** metaproc still sums RSS over process trees and has no leading
  memory-pressure signal on macOS
  ([`metaproc/docs/memory-accounting-reference.md:196-212`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/memory-accounting-reference.md#L196-L212)).

This supports urollup’s deferral of resource observations, gives a future collector its
rules, and bears on urollup’s own benchmarks: the plan gates peak RSS from `getrusage`,
which on macOS omits compressed pages that `phys_footprint` counts.

### Ledgers, Execution Model and Formats

- **Ledger first:** one parse feeds a reconciled event ledger; producer IDs win,
  byte-equivalent duplicates collapse, conflicting duplicates fail, unowned evidence
  goes to an `unattributed` node or the root but never both, and every projection,
  including recovery, is rebuilt from the ledger rather than from cached totals
  ([`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:113-125`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L113-L125),
  [`metaproc/docs/arch/arch-metaproc-core.md:2419-2470`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-metaproc-core.md#L2419-L2470)).
- **Facts and projections:** the execution model keeps durable facts separate from
  rebuildable projections, re-derives commands idempotently from state, and proves
  replay determinism with property tests that are themselves verified by mutation
  ([`metaproc/docs/arch/arch-execution-model.md:44-67`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-execution-model.md#L44-L67),
  [`metaproc/docs/arch/arch-execution-model.md:94-129`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-execution-model.md#L94-L129)).
- **Commit and fencing:** the task is the unit of completion, the attempt the unit of
  execution and the commit the unit of trust; outputs stay private until one atomic
  commit, and a late attempt whose lease was reclaimed is refused at commit time because
  reclaiming a lease does not stop its holder
  ([`metaproc/docs/process-framework-concepts.md:353-434`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/process-framework-concepts.md#L353-L434),
  [`metaproc/docs/execution-model-design.md:156-182`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/execution-model-design.md#L156-L182)).
- **Key spaces:** a key is meaningless without the identity domain it belongs to, so
  every roster records a key-space identifier
  ([`metaproc/docs/execution-model-design.md:49-75`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/execution-model-design.md#L49-L75)).
- **Scale guard:** recomputing whole state per event is quadratic over a run, invisible
  at test size, so a scale test guards about 0.03 seconds per pass over 2,400 tasks
  ([`metaproc/docs/arch/arch-execution-model.md:131-146`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-execution-model.md#L131-L146)).
- **Artifacts:** JSONL streams, YAML state, one committed compiled schema for
  `metaproc:ResourceUsageSummary` with a drift test, and `usage.md`
  (`metaproc:UsageReport/0.2`), which holds totals by variant, model and provider with
  no extents, currency or pricing basis
  ([`metaproc/docs/artifact-catalog.md:14-68`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/artifact-catalog.md#L14-L68),
  [`metaproc/src/metaproc/models/usage.py:111-128`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/models/usage.py#L111-L128),
  [`metaproc/tests/test_resource_finalization.py:155-201`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_resource_finalization.py#L155-L201)).
  Nothing in it can merge, and it shares no structure with `urollup:UsageSummary/v1`.
- **Trace IDs:** span IDs are SHA-1 digests over the source name and the log’s absolute
  path
  ([`metaproc/src/metaproc/trace/ids.py:22-37`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/ids.py#L22-L37)),
  so copying a run directory changes every ID.
- **Ledger readers disagree:** the runpool reader skips bad lines and the resource
  ledger reader fails on them
  ([`metaproc/src/metaproc/runpool/event_reader.py:24-54`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/runpool/event_reader.py#L24-L54),
  [`metaproc/src/metaproc/logutil/resource_events.py:70-84`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/resource_events.py#L70-L84)).
- **YAML dates:** unquoted dates parsed as date objects caused 8 of 12 contract failures
  across 504 artifacts in one downstream pipeline
  ([`metaproc/docs/project/specs/active/plan-2026-08-20-contract-failure-primitives.md:94-116`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/active/plan-2026-08-20-contract-failure-primitives.md#L94-L116)).

### Performance Lessons

[`metaproc/docs/performance-notes.md:21-142`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L21-L142)
distills measured lessons from Metabrowser, which reads the same logs:

- **Cold and warm differ:** report both, with full distributions; a tree view took 19.9
  seconds cold against 4.0 warm, and discovery reached 132 seconds under heavy dispatch
  I/O
  ([`metaproc/docs/performance-notes.md:365-376`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L365-L376)).
- **Scope beats speed:** narrowing discovery to `.logs/` and `.state/` cut a poll from
  976 ms to 5 ms, while switching to `scandir` alone gave 2x
  ([`metaproc/docs/performance-notes.md:53-66`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L53-L66)).
- **One read:** sniffing the first 20 lines, then reopening, read a 38 MB log three
  times; buffering the sniff and replaying it through the parser fixed it
  ([`metaproc/docs/performance-notes.md:92-101`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L92-L101)).
- **Cap payloads at the boundary:** an 8 KiB per-event cap on raw payloads in files over
  2 MiB cut one response from 41 MB to 5.7 MB, but a 256 KiB per-line cap drops
  oversized lines entirely
  ([`metaproc/docs/performance-notes.md:103-117`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L103-L117)),
  which accounting must report instead.

### Filesystem, Locking and Resume

- **Atomic writes:** strif’s `atomic_output_file` writes a temp file in the same
  directory and renames it, with no fsync, no no-clobber mode, and permissions set after
  creation
  ([`metaproc/src/metaproc/io/state_io.py:41-51`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/state_io.py#L41-L51),
  [`metaproc/src/metaproc/dispatch/credential_pool.py:896-911`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/credential_pool.py#L896-L911)).
- **Verified publish:** `gzip_file` hashes the source while compressing, fsyncs,
  decompresses to compare length and SHA-256, re-stats the source to catch a live
  writer, and only then renames
  ([`metaproc/src/metaproc/commands/gzip_text.py:202-326`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/commands/gzip_text.py#L202-L326)).
  Its temp name is fixed, so concurrent sweeps collide.
- **Locks:** a grep test bans `flock` and `fcntl` for NFS safety
  ([`metaproc/tests/test_locking_policy.py:9-61`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_locking_policy.py#L9-L61)),
  and `mkdir` locks are reclaimed by age with no owner token
  ([`metaproc/src/metaproc/io/mkdir_lock.py:125-183`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/mkdir_lock.py#L125-L183)),
  so two contenders can both reclaim and a holder whose lock was reclaimed deletes its
  successor’s. The orchestrator lease adds an owner token and host and PID checks
  ([`metaproc/src/metaproc/io/orchestrator_lease.py:204-280`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/orchestrator_lease.py#L204-L280)).
- **Resume:** a step is complete only with a completed status, valid outputs and a
  matching fingerprint over the resolved step and referenced prompt bytes
  ([`metaproc/src/metaproc/engine/dep_state.py:50-98`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/dep_state.py#L50-L98)).
  The mtime cache stats a file after deriving its value, so a concurrent write can be
  cached under a fresh stamp
  ([`metaproc/src/metaproc/utils/mtime_cache.py:74-109`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/utils/mtime_cache.py#L74-L109)).
- **Archives:** extraction validates the whole tar first, rejecting empty, absolute,
  `..` and duplicate names, links, special files and escaping paths, with member and
  size caps
  ([`metaproc/src/metaproc/io/safe_archive.py:12-90`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/safe_archive.py#L12-L90),
  [`metaproc/tests/io/test_safe_archive.py:37-85`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/io/test_safe_archive.py#L37-L85)).
  Sizes come from headers rather than decompressed bytes, and backslashes, drive letters
  and case-fold duplicates are not checked.

### Supply Chain and Engineering Practice

- **Cool-off:** `uv.toml` sets `exclude-newer = "14 days"` with per-package date
  exemptions
  ([`metaproc/uv.toml:1-11`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/uv.toml#L1-L11)),
  `.npmrc` sets `min-release-age=14`, `ignore-scripts` and `save-exact`
  ([`metaproc/.npmrc:1-19`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/.npmrc#L1-L19)),
  and the Makefile forces the uv config file and frozen runs
  ([`metaproc/Makefile:7-18`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/Makefile#L7-L18)).
- **Gate:**
  [`metaproc/devtools/check_supply_chain.py:12-124`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/devtools/check_supply_chain.py#L12-L124)
  checks npm settings and exact pins, registry integrity hashes, the uv cool-off,
  SHA-pinned actions and trusted-publishing constraints.
  It does not check workflow permissions, lockfile agreement with `uv.toml`, or waiver
  expiry, and the policy text has drifted: it names `softschema==0.4.0` while `uv.toml`
  admits 0.6.0, and keeps an advisory waiver past its stated removal date
  ([`metaproc/SUPPLY-CHAIN-SECURITY.md:40-83`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/SUPPLY-CHAIN-SECURITY.md#L40-L83)).
- **CI and release:** CI uses read-only permissions and `persist-credentials: false`
  ([`metaproc/.github/workflows/ci.yml:1-40`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/.github/workflows/ci.yml#L1-L40));
  publishing runs only on a release, checks the tag against the version and uses trusted
  publishing
  ([`metaproc/.github/workflows/publish.yml:1-61`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/.github/workflows/publish.yml#L1-L61)),
  with attestations deferred
  ([`metaproc/TODO.md:12-16`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/TODO.md#L12-L16)).
- **Public hygiene:**
  [`metaproc/devtools/public_hygiene.py:41-115`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/devtools/public_hygiene.py#L41-L115)
  scans tracked files, built distributions and commit messages for hashed banned tokens,
  private emails, home paths and common token formats, but has no patterns for `sk-ant-`
  or `AIza` keys, JWTs, UUIDs, session IDs or macOS temporary paths.
- **Fixture synthesis:**
  [`metaproc/devtools/synthesize_fixtures.py:17-208`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/devtools/synthesize_fixtures.py#L17-L208)
  rewrites prose, ID-like keys, paths, emails and dates idempotently
  ([`metaproc/tests/test_synthetic_fixtures.py:17-38`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_synthetic_fixtures.py#L17-L38)).
  Its date pattern misses full datetimes, so capture timestamps survive, as do session
  UUIDs, `msg_` and `toolu_` IDs, epoch timestamps, thinking signatures and the rewrite
  header’s original file size.
- **Goldens:** a missing golden is written and the test skipped
  ([`metaproc/tests/test_runpool_golden.py:310-316`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_runpool_golden.py#L310-L316)),
  so deleting a golden still passes CI.

### Documentation Conflicts With Code

These stale statements matter if urollup ports from the documents rather than the code:

- **Rate-limit source:** the core architecture and adapter runbook say the Pi parser
  produces `rate_limit_event` records binned by provider
  ([`metaproc/docs/arch/arch-metaproc-core.md:2268-2316`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-metaproc-core.md#L2268-L2316),
  [`metaproc/docs/runbooks/adapter-compatibility.runbook.md:138-168`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/runbooks/adapter-compatibility.runbook.md#L138-L168)),
  but only the Claude parser emits them, with the provider pinned to `anthropic`
  ([`metaproc/src/metaproc/logutil/parsing.py:203-220`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L203-L220)).
- **Fan-out memory:** the memory reference says fan-out cost is a sum over process trees
  ([`metaproc/docs/memory-accounting-reference.md:16-25`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/memory-accounting-reference.md#L16-L25)),
  while the rollup takes the maximum across siblings
  ([`metaproc/src/metaproc/engine/resource_rollup.py:800-816`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/resource_rollup.py#L800-L816)).
- **Memory gauge:** the runpool benchmark section sizes concurrency from
  `kern.memorystatus_level` and says it accounts for inactive pages
  ([`metaproc/docs/arch/arch-runpool.md:488-492`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-runpool.md#L488-L492)),
  while the same document calls it an alarm that never sizes anything
  ([`metaproc/docs/arch/arch-runpool.md:112-127`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-runpool.md#L112-L127)).
- **Slot paths:** the harness and authentication documents give two different slot paths
  ([`metaproc/docs/arch/arch-claude-code-harness.md:189-193`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-claude-code-harness.md#L189-L193),
  [`metaproc/docs/arch/arch-authentication.md:898-902`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-authentication.md#L898-L902)),
  and the code uses a third
  ([`metaproc/src/metaproc/dispatch/slot_coordinator.py:141-146`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/slot_coordinator.py#L141-L146)).

### Fixtures and Tests

| Fixture or test | Content | Use in urollup |
| --- | --- | --- |
| `metaproc/tests/fixtures/trace_agents/` captures and sidecars | Rewritten Claude 2.1.126, Codex and Pi captures of 11 to 19 lines with real counters | Parser cases after re-scrubbing; unsuitable for prefix, tail or ordering tests |
| `metaproc/tests/fixtures/log_compaction/` | Unrewritten Claude 2.1.81 stream with hook records and a rate-limit event; pre-0.84 Pi stream with every streaming event type | Pi usage-revision dedupe; detection after hook records |
| `metaproc/tests/fixtures/claude_api_signals/` | 200, 401 and 429 results, an OAuth refresh failure, and error keywords inside tool output | Error results without usage; hostile text |
| [`metaproc/tests/test_usage.py:384-443`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_usage.py#L384-L443) and [`metaproc/tests/test_usage.py:806-846`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_usage.py#L806-L846) | `modelUsage` precedence; explicit zero cost | Golden cases with urollup’s own expected values |
| [`metaproc/tests/test_resource_reconciliation.py:40-160`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_resource_reconciliation.py#L40-L160) | Duplicate and conflicting events | Reconciliation property cases |
| [`metaproc/tests/io/test_safe_archive.py:37-85`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/io/test_safe_archive.py#L37-L85) | Archive rejection matrix | Bundle reader negative tests |
| [`metaproc/tests/test_log_compaction.py:51-400`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_log_compaction.py#L51-L400) | Pi and Codex log rewrite rules and idempotence | Recognizing rewritten input |

## qm: A Multiplayer Agent Harness

This section covers qm only; its overlaps with metaproc and squares are reconciled
[below](#reconciliation-with-other-urollup-research).

### Status, License and Provenance

- **What it is:** a harness that gives each person and room in a company scoped memory,
  files, credentials and sandboxes, reached through Slack and a web UI, with Pi,
  OpenCode, Codex and Claude Code all driving one core
  ([`qm/README.md:1-30`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/README.md#L1-L30)).
- **Maturity:** its security policy calls it early, experimental software
  ([`qm/SECURITY.md:1-5`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/SECURITY.md#L1-L5));
  `package.json` marks version 0.1.0 and private.
- **Status here:** the snapshot is a shallow clone at `78dd4cc` (2026-08-03) inside
  metaproc’s ignored `attic/` directory; the public repository has moved on, so any port
  should be rechecked against upstream.
  Its `adrs/` directory holds only a placeholder, because contributions arrive as
  human-written proposals there
  ([`qm/CONTRIBUTING.md:1-12`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/CONTRIBUTING.md#L1-L12)),
  so there are no ADRs to review.
- **License:** MIT, copyright “QM contributors”
  ([`qm/LICENSE:1-3`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/LICENSE#L1-L3));
  porting keeps that notice.

### Harness Adapters and Session Capture

Every adapter runs agents inside a throwaway jail with native persistence off, so no
agent transcript, rollout or session file survives a turn:

| Agent | Integration | Persistence and config | Usage source | Handles kept |
| --- | --- | --- | --- | --- |
| Claude Code | Agent SDK 0.3.211 `query()` with `includePartialMessages` ([`qm/src/harness/claude-harness.ts:455-480`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/claude-harness.ts#L455-L480)) | `persistSession: false`; `HOME` and `CLAUDE_CONFIG_DIR` inside the jail ([`qm/src/harness/claude-harness.ts:121-127`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/claude-harness.ts#L121-L127)) | `assistant.message.usage` merged by `message.id`; `result.total_cost_usd` | None; history is re-sent as text each turn |
| Codex | One `codex app-server` over JSON-RPC; a new `thread/start` per turn, with history replayed through `thread/inject_items` ([`qm/src/harness/codex-harness.ts:598-650`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/codex-harness.ts#L598-L650)) | `ephemeral: true`; `CODEX_HOME` inside the jail ([`qm/src/harness/codex-harness.ts:188-197`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/codex-harness.ts#L188-L197)) | `thread/tokenUsage/updated` running totals per thread ([`qm/src/harness/codex-harness.ts:436-452`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/codex-harness.ts#L436-L452)) | Thread ID in memory only |
| Pi | In-process library with `SessionManager.inMemory()` and temporary agent directories ([`qm/src/harness/pi-harness.ts:997-1030`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/pi-harness.ts#L997-L1030)) | No session files | `message_end` usage and `cost.total` ([`qm/src/harness/pi-harness.ts:527-546`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/pi-harness.ts#L527-L546)) | None |
| OpenCode | `opencode serve` 1.17.18 with a bridging plugin; sessions deleted after the turn | XDG data, config and cache inside the jail ([`qm/src/harness/opencode-harness.ts:695-710`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/opencode-harness.ts#L695-L710)) | `info.tokens` with `reasoning` and `cache`, and `info.cost` ([`qm/src/harness/opencode-harness.ts:376-393`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/opencode-harness.ts#L376-L393)) | Child sessions by `parentID` |

qm’s durable record is its own Postgres tables
([`qm/src/sessions/postgres-session-store.ts:172-227`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/sessions/postgres-session-store.ts#L172-L227)):

- **Tape:** `session_tape` rows hold each harness’s native messages (Claude SDK messages
  with image bytes stripped, Codex completed items without usage, Pi agent messages, and
  OpenCode messages with tokens), tagged with a `transcriptFormat` such as
  `claude-agent-sdk` or `responses-api`
  ([`qm/src/sessions/session-store.ts:35-60`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/sessions/session-store.ts#L35-L60),
  [`qm/src/harness/harness.ts:152-161`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/harness.ts#L152-L161)).
- **Requests:** `session_llm_requests` rows hold the captured request, `ttft_ms`,
  `duration_ms`, `step_gap_ms`, per-tool wall times, `usage_json` and `gap_phases_json`
  per step
  ([`qm/src/sessions/session-store.ts:109-193`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/sessions/session-store.ts#L109-L193)).
  Request capture is on by default and admins can read captured requests
  ([`qm/SECURITY.md:118-130`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/SECURITY.md#L118-L130)).
- **Participants:** principals attach to sessions through validity windows, and turns
  are attributed per principal and day
  ([`qm/src/sessions/session-store.ts:195-209`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/sessions/session-store.ts#L195-L209)).

### Per-Request Usage and Timing Records

- **One shape, four meanings:** `LlmCallUsage` holds `input`, `output`, `cacheRead`,
  `cacheWrite`, `totalTokens` and `costUsd` for every harness
  ([`qm/src/sessions/session-store.ts:109-116`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/sessions/session-store.ts#L109-L116)),
  but `input` includes cached tokens for Codex and excludes them elsewhere,
  `totalTokens` is input plus output for Codex and adds both cache counts for Claude,
  OpenCode folds `reasoning` into `totalTokens` only, and Codex `cacheWrite` and
  `costUsd` are literal zeros
  ([`qm/src/harness/codex-harness.ts:133-143`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/codex-harness.ts#L133-L143)).
- **Claude content blocks:** repeated assistant records are merged by `message.id`,
  taking each field’s maximum
  ([`qm/src/harness/claude-harness.ts:579-608`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/claude-harness.ts#L579-L608)),
  and a fixture proves one call from repeated blocks
  ([`qm/test/claude-harness-turn.test.ts:147-180`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/test/claude-harness-turn.test.ts#L147-L180)).
- **Claude cumulative cost:** with streaming input, one SDK query yields a `result` per
  steered prompt, and `total_cost_usd` is cumulative across them, so qm charges each
  step the difference from the previous total
  ([`qm/src/harness/claude-harness.ts:531-573`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/claude-harness.ts#L531-L573),
  [`qm/test/claude-harness-turn.test.ts:214-250`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/test/claude-harness-turn.test.ts#L214-L250)).
- **Claude inclusion bug:** when no per-message usage was seen, qm computes uncached
  input as `input_tokens` minus both cache counts
  ([`qm/src/harness/claude-harness.ts:756-790`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/claude-harness.ts#L756-L790)),
  although Anthropic’s `input_tokens` already excludes them.
- **Codex running totals:** qm keeps the latest `tokenUsage.total` per thread, sums
  threads (parent and multi-agent children) into one row per turn, and counts a model
  call only when the input total strictly increases, using `last` for its size
  ([`qm/src/harness/codex-harness.ts:145-169`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/codex-harness.ts#L145-L169),
  tested in
  [`qm/test/codex-harness.test.ts:233-244`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/test/codex-harness.test.ts#L233-L244)).
  In Codex source a thread’s total counts only its own responses, never its children’s,
  but a child forked with its parent’s history (a legacy-mode subagent or a user fork)
  starts from the parent’s total, and the app-server replays that total to the fork
  ([`codex-rs/core/src/session/mod.rs:1486-1493`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1486-L1493),
  [`codex-rs/app-server/tests/suite/v2/thread_fork.rs:1157-1208`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/app-server/tests/suite/v2/thread_fork.rs#L1157-L1208)),
  so summing per-thread totals can count the inherited part twice.
- **Pi alignment:** usage rows are matched by index to captured request payloads, and a
  count mismatch stores no usage rather than misattributing it
  ([`qm/src/harness/pi-harness.ts:1611-1625`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/pi-harness.ts#L1611-L1625)).
- **Seeded history:** replayed Pi assistant messages carry all-zero usage and cost
  ([`qm/src/harness/replay.ts:110-119`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/replay.ts#L110-L119)),
  because pi-ai sizes its output limit from the last assistant message’s usage and a
  stale value clamped `max_tokens` to 1
  ([`qm/test/pi-harness-output-guard.test.ts:162-189`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/test/pi-harness-output-guard.test.ts#L162-L189)).
- **Timing per step:** time to first token, duration, the gap since the previous call,
  tool wall times, and a gap decomposed into named phases (provisioning, credentials,
  recall, tool body, persistence, stream open and others) by unioning intervals per
  phase and reporting an explicit `residual`
  ([`qm/src/harness/pi-harness.ts:396-526`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/pi-harness.ts#L396-L526)).

### Cost, Budgets and Rate Limits

- **No price table:** cost is Pi’s catalog calculation from a model registry that clones
  newer model IDs from older templates with overridden prices, derives cache reads as a
  tenth of input and defaults cache writes to zero
  ([`qm/src/model/pi-models.ts:42-92`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/model/pi-models.ts#L42-L92),
  [`qm/src/model/pi-models.ts:128-146`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/model/pi-models.ts#L128-L146)).
- **Budgets from estimates:** one rolling-window USD cap per principal plus an org cap,
  both unlimited by default, charge `inputTokens / 1e6 × 5` with output uncharged, cache
  reads at full input price, and pre-call tokenizer estimates for Pi and OpenCode
  ([`qm/src/ratelimit/budget.ts:13-50`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/ratelimit/budget.ts#L13-L50),
  [`qm/src/model/pi-models.ts:252`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/model/pi-models.ts#L252),
  [`qm/src/harness/pi-harness.ts:1500-1516`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/pi-harness.ts#L1500-L1516)).
- **Uncounted side calls:** detection, harness summary (qm’s compaction) and security
  screening record estimated model calls; one-shot, judge, title and memory calls record
  nothing
  ([`qm/src/harness/harness.ts:37-42`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/harness.ts#L37-L42),
  [`qm/src/harness/harness.ts:139-149`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/harness.ts#L139-L149)).
- **No provider limits:** qm has its own per-principal request limiter and reads no
  Codex `rate_limits`, Claude limit events or rate-limit headers.
- **Fast mode is a separate tier:** fast mode became opt-in because an unset flag billed
  turns against a tier an organization may have no quota for, which the provider reports
  as zero fast-mode input tokens per minute; qm sets `speed: "fast"` and one-hour cache
  control on Anthropic requests
  ([`qm/src/harness/pi-harness.ts:1037-1066`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/pi-harness.ts#L1037-L1066)).

### Cache Metrics and Observability

- **Cache hit ratio:** `cacheRead / (cacheRead + cacheWrite + uncachedInput)`, `null`
  when all three are absent or the denominator is zero, plus a stable-prefix-miss flag
  when writes are at least 1,024 tokens and the ratio is under 0.1
  ([`qm/src/admin/metrics-sink.ts:41-62`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/admin/metrics-sink.ts#L41-L62)).
  The admin API reports both a per-turn average and a pooled ratio.
- **Double count in the UI:** the admin page computes real input as
  `cacheRead + cacheWrite + input` for every row
  ([`qm/plugins/admin/public/index.html:13795-13824`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/plugins/admin/public/index.html#L13795-L13824)),
  which counts cached tokens twice for Codex rows.
- **Lost metrics:** when an orchestrator nudge runs a second harness turn, the result
  object is replaced and the first run’s model calls and cache usage drop out of the
  turn metrics
  ([`qm/src/core/orchestrator.ts:2366-2374`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/core/orchestrator.ts#L2366-L2374),
  [`qm/src/core/orchestrator.ts:2583-2589`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/core/orchestrator.ts#L2583-L2589)).

### Resources, Agent Memory and Token Estimation

- **Resources:** sandboxes set CPU and memory limits, but nothing measures CPU or memory
  use, consistent with urollup’s deferral.
- **Agent memory:** “memory” in qm is agent notebook memory with an LLM-judged
  benchmark; its model calls go through uncounted one-shot paths.
- **Token estimation:** `countTokens` uses an Anthropic tokenizer for every provider
  over 4,000-character chunks and extrapolates from the first 64,000 characters
  ([`qm/src/util/tokens.ts:1-16`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/util/tokens.ts#L1-L16)),
  and its harness summary thresholds use it.

### qm Lessons for urollup

- **Harness ledgers are a separate source:** a multi-tenant harness deliberately
  discards native logs, so its usage can reach urollup only through an export of its own
  rows, each read with its harness’s token semantics.
- **Normalize at ingest:** one field name with per-agent inclusion rules produced a
  double count and a wrong hit ratio; urollup’s disjoint categories must be computed in
  each adapter and the native inclusive value kept beside them.
- **Cumulative fields hide in stream results:** Claude’s `total_cost_usd`, Codex’s
  `tokenUsage.total` and `codex-exec` `turn.completed.usage` all need delta or
  latest-value handling.
- **Seeded copies look like requests:** harness-seeded history with zero usage is not a
  request, and copied usage can mislead a model’s own context sizing.
- **Estimates must never enter the ledger:** qm’s budgets run on pre-call estimates and
  a flat input price, which urollup’s separation of measured, list-price and
  source-reported values already forbids.
- **Every call path needs accounting:** side calls for screening, titles, memory and
  judging consume tokens outside the main transcript, like the Haiku usage visible only
  in `claude-stream` `modelUsage`.

## Reconciliation With Other urollup Research

### With the squares Review

The [squares review](research-2026-09-14-squares-code-review.md) (bead `uro-o53w`)
covers persistent logs, which metaproc and qm never read, so most findings complement
each other. This table lists where they touch, with the reconciled position and the
review that holds the detail; Codex facts come from Codex source at `6b9826e`.

| Topic | squares review | metaproc and qm | Reconciled position |
| --- | --- | --- | --- |
| Codex `token_count` and running totals | Sums `last_token_usage` per event, overcounting repeated snapshots and `info: null` events; detail on synthetic estimates, context-window resets and `token_usage_record` lives there | qm keeps the latest app-server total per thread and counts a call only when the input total rises; `codex-exec` `turn.completed.usage` is also a cumulative per-thread total | Agreement: squares recommendation 2 and qm’s rule express one policy for rollouts, app-server streams and exec captures; prefer `token_usage_record` where present (0.153+), and subtract a forked child’s inherited total |
| Legacy Codex subagent replay | Ordered child-boundary rule, with tested heuristics as the last tier; detail lives there | metaproc reads no rollouts; qm replays history into fresh ephemeral threads | No overlap; adopt squares recommendation 1 |
| Codex capture and rollout overlap | Not covered | metaproc’s `codex exec` writes both a capture and a rollout; detail lives here | Addition: `thread.started.thread_id` equals rollout `session_meta.id`; the rollout owns usage and the capture is a check (P2) |
| Claude content blocks | squares sums every `assistant` record (confirmed overcount) | metaproc reads only the final `result`, which loses killed attempts; qm merges by `message.id` with field-wise maximum | Reconcile by request, select one block record per request deterministically with a diagnostic when records disagree (about 13% do on `output_tokens` in transcripts), never merge fields, and use `result` totals as checks (C1) |
| Model and tool time | Claude transcripts record no model stream timing | `claude-stream` `result` has `duration_ms` and `duration_api_ms`; qm records per-step time to first token and duration | Addition: captured streams and harness rows give invocation-level or per-step API time that transcripts lack; record it as source-reported, never per request unless the source is per request |
| Tool intervals | `tool_use` to `tool_result` includes scheduling and permission waits | metaproc publishes the same interval as tool latency | **Disagreement, resolved:** the interval includes scheduling and permission waits, so squares is right; both reviews and C2 name it a tool interval, never latency or execution time |
| Nested and parallel time | Overlap-safe definitions: agent-active seconds, busy union, parallel overlap, exclusive categories by priority; detail lives there | metaproc sums nested `wall_time_s` and takes peak RSS across concurrent siblings; qm unions intervals per gap phase and reports an explicit residual | squares’ definitions replace metaproc’s, whose defects become negative tests; add qm’s explicit residual so unattributed time is visible |
| Compaction | Claude `compact_boundary` records, Codex `ContextCompaction` items and their `token_count` estimates compact an agent’s context | metaproc’s `type: compaction` header marks a rewritten log file; qm’s compaction appends a summary entry to its own session | Three unrelated things share one word; both reviews and urollup diagnostics call them context compaction, log rewrite and harness summary |
| Tool categories | Structural shell command identities computed before stripping | metaproc’s family, operation and role table across agents | Complementary: metaproc’s table for categories and squares’ resolver for shell identities, both before the strip policy runs (squares recommendation 4) |
| Account, plan and budget handling | None; squares prices nothing and budgets in wall minutes | metaproc labels, quota groups and lease joins; qm per-principal USD budgets from estimates; neither reads Codex `rate_limits` `limit_id`, `plan_type` or `credits`; detail lives here | Pricing stays optional (squares insight 9); accounts need explicit evidence such as lease events or recorded plan fields; people (principals) and billing accounts are different properties |
| Completeness | Open and abandoned Codex turns at a snapshot need flags | Killed attempts with no terminal record; `turn.failed` without usage and interrupted `codex exec` turns with no event; qm writes a no-usage row for failed turns | One per-extent completeness field covers all of these (squares recommendation 6 plus C1) |
| Fixtures | Synthetic Codex builders, safe to port | metaproc captures with residual real IDs; qm fixtures are synthetic unit-test streams | Port squares’ builders and qm’s small stream cases first, and metaproc captures after re-scrubbing |

### With the Portable Brief’s Log Dialect Survey

Corrections and additions from metaproc and qm to the log dialect survey in the
[portable research brief](research-2026-09-13-portable-agent-usage.md), as the survey
stood on 2026-09-13. Facts from the Codex, Pi, ccusage and session-report source reviews
are folded into that brief directly and appear here only where they change a row.

| Dialect or topic | Survey on 2026-09-13 | Correction or addition |
| --- | --- | --- |
| `claude-stream` records | `system` `init`, `assistant` and `user`, and a final `result` | Add `rate_limit_event` and, with hooks configured, `system` `hook_started` and `hook_response` before `init`, whose hook output the strip policy must stub |
| `claude-stream` timestamps and IDs | Not stated | Only `user` records carry `timestamp`; assistant records carry `message.id` but no `requestId`; `init` carries `claude_code_version` and `apiKeySource` |
| `claude-stream` result | `usage`, `modelUsage`, `total_cost_usd`, `num_turns`, `duration_ms` | Add `duration_api_ms`, `terminal_reason`, `api_error_status`, `permission_denials` and `usage.iterations`; top-level `usage` covers the main model while `modelUsage` covers every model; with streaming input, `total_cost_usd` is cumulative across several `result` records (qm) |
| `claude-stream` subagents | `parent_tool_use_id` listed | Subagent messages arrive inline in the parent stream and must be attributed by `parent_tool_use_id`, which qm does not do |
| `codex-exec` usage | `turn.completed` has `usage` | Fields are `input_tokens`, `cached_input_tokens`, `output_tokens` and, in some releases, `reasoning_output_tokens` and `cache_write_input_tokens`, with no `total_tokens`; the value is the thread’s cumulative total, excluding subagents; `turn.failed` has no usage and interrupted turns emit nothing; `error` events can be non-terminal reconnect notices; items use `item.type`, and 0.124.0 used `item.item_type` |
| `codex-exec` overlap | Explicit input only | `codex exec` also writes a rollout unless `--ephemeral` is passed (present in codex-cli 0.135.0 help), so captures and rollouts overlap by default; `thread.started.thread_id` equals the rollout’s `session_meta.id` |
| Codex app-server | Not surveyed | A third Codex surface: `thread/tokenUsage/updated` with camelCase `tokenUsage.total` and `last`; `ephemeral: true` threads write no rollout |
| `pi-events` usage | Lists event types | Usage repeats across five event types: `message_start` is zero, `message_update` is partial (its shape changed at Pi 0.84.0 and 0.84.2), `message_end` is the only final revision, with `responseId`, and `turn_end` and `agent_end.messages[]` are copies; metaproc’s `message_final` is a harness record, not a Pi event; tool events have no timestamps |
| `pi-session` and `pi-events` cost | `cost` is a breakdown computed by Pi | `cost.total` is 0 for providers Pi does not price, and a harness catalog can supply the rates (qm clones model IDs with its own prices); harness-seeded assistant messages can carry all-zero usage |
| Captured dialect storage | Compression mentioned only for Codex rollouts | Harnesses gzip captures and may prepend a rewrite header and drop streaming events |
| Current-session signals | Nested agents inherit variables | Confirmed for metaproc, which strips only `CLAUDECODE` and `CLAUDE_CODE_ENTRYPOINT`; both harnesses scope `CLAUDE_CONFIG_DIR` and `CODEX_HOME` per attempt or turn, which relocates or discards persistent logs; Codex tools get their own thread’s `CODEX_THREAD_ID`, while Codex hooks get no `CODEX_*` identity of their own and a root `session_id` |
| Provider windows | Claude `quotaLimits` on some assistant entries | `claude-stream` `rate_limit_event` uses `rateLimitType` values such as `five_hour`, epoch-second `resetsAt` and overage fields, without timestamps; whether transcript `quotaLimits` shares these values is unverified |
| Model identity | `message.model` and `turn_context` | Claude Code and Gemini CLI silently fall back from unknown requested models, so a requested model in `argv` or a sidecar is a configured value, never evidence; Codex rollouts also record only the requested model |

## Reuse Table

Reuse modes: **port code** (translate the implementation into Rust, recording the source
commit), **port tests** (rewrite the cases as urollup fixtures and goldens), **reuse
fixtures** (copy data after scrubbing), **share format** (recognize or adopt a format),
**learn only** (a lesson or pitfall).

| Item | Where | Reuse mode | urollup bead or plan section | Notes and risks |
| --- | --- | --- | --- | --- |
| `modelUsage` precedence over top-level `usage` | [`metaproc/src/metaproc/logutil/usage.py:239-301`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L239-L301); [`metaproc/tests/test_usage.py:384-443`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_usage.py#L384-L443) | Port code; port tests | `uro-y2qj`; contracts: Measure Contracts | Use as a per-model reconciliation check with an unitemized residual (C1) |
| `rate_limit_event` record shapes | [`metaproc/tests/fixtures/trace_agents/claude-sample.jsonl:3`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/trace_agents/claude-sample.jsonl#L3); [`metaproc/tests/fixtures/log_compaction/claude_sample.jsonl:13`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/log_compaction/claude_sample.jsonl#L13) | Reuse fixtures | `uro-y2qj`, `uro-eamm`; contracts: Entities | Keep every `rateLimitType` and `status` verbatim; metaproc’s type filter, `blocked` check and merge order are bugs |
| `codex-exec` inclusion rules | [`metaproc/src/metaproc/logutil/usage.py:304-327`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/usage.py#L304-L327); [`metaproc/tests/fixtures/trace_agents/codex-sample.jsonl:11`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/trace_agents/codex-sample.jsonl#L11) | Port code; reuse fixtures | `uro-y2qj` | `turn.completed.usage` is a cumulative per-thread total, so per-turn figures are differences; `turn.failed` has no usage and interrupted turns emit nothing |
| Dialect detection by record type | [`metaproc/src/metaproc/logutil/parsing.py:1498-1574`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L1498-L1574); [`metaproc/src/metaproc/trace/extractors/codex_agent.py:511-537`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/codex_agent.py#L511-L537) | Port code; port tests | `uro-y2qj`; plan: Sources and snapshot boundary | Buffer the sniffed lines and replay them; extend the window past hook records and headers |
| Log rewrites: header, dropped events, synthetic `message_final`, gzip | [`metaproc/src/metaproc/logutil/compaction.py:236-630`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/compaction.py#L236-L630); [`metaproc/src/metaproc/io/gz_io.py:37-104`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/gz_io.py#L37-L104) | Share format | `uro-y2qj`, `uro-qnok`; contracts: Source Snapshots | The header is metadata, not a record; a `.gz` sibling replaces the plain file |
| Invocation sidecar | [`metaproc/src/metaproc/runpool/backend.py:34-115`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/runpool/backend.py#L34-L115) | Share format | `uro-y2qj`; contracts: Capture Layers | `argv` model and effort are configured values; `argv` holds prompt text and the environment is redacted only by name, so never capture either verbatim |
| Pi usage repeated across stream event types | [`metaproc/src/metaproc/logutil/parsing.py:754-918`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L754-L918); [`metaproc/tests/fixtures/log_compaction/pi_sample.jsonl:1-80`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/fixtures/log_compaction/pi_sample.jsonl#L1-L80) | Reuse fixtures; learn | `uro-qnok` | Take only `message_end` as final, keyed by `responseId`; bound `message_update` parsing by Pi version; summing `agent_end` alone loses crashed attempts |
| Cross-agent tool taxonomy and tool identity | [`metaproc/src/metaproc/trace/extractors/common.py:96-200`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/common.py#L96-L200); [`metaproc/src/metaproc/trace/extractors/codex_agent.py:357-402`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/extractors/codex_agent.py#L357-L402); [`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:154-172`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L154-L172) | Port code | `uro-d135`; plan: CLI and report contracts | Keep native tool names beside categories; terminal tool counts add only a positive residual |
| Substring prefilter before JSON parsing | [`metaproc/src/metaproc/engine/runtime.py:187-203`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/runtime.py#L187-L203) | Port code | `uro-g8r0`, `uro-y2qj` | Confirm the top-level type after a match |
| Coverage states with lineage | [`metaproc/src/metaproc/models/resources.py:85-183`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/models/resources.py#L85-L183) | Port code | `uro-spce`; contracts: Measure Contracts | Pair with a rule that any unknown member makes a group partial |
| Ledger reconciliation rules and cases | [`metaproc/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md:113-125`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/project/specs/done/plan-2026-08-03-focused-resource-observability.md#L113-L125); [`metaproc/tests/test_resource_reconciliation.py:40-160`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_resource_reconciliation.py#L40-L160) | Port tests | `uro-spce` | metaproc’s merge raises on overlap; urollup’s must stay idempotent |
| Mutation-verified property tests and scale guard | [`metaproc/docs/arch/arch-execution-model.md:111-146`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-execution-model.md#L111-L146) | Learn; port test design | `uro-spce`, `uro-sbnk` | A scale test catches accidental quadratic reconciliation |
| Price table notes and review-date test | [`metaproc/src/metaproc/data/pricing.md:1-282`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/data/pricing.md#L1-L282); [`metaproc/tests/test_usage.py:286-331`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/test_usage.py#L286-L331) | Port tests; learn | `uro-wuby`; contracts: Price Table | Floats, no effective dates, deleted history and suffix lookup; the notes make good repricing scenarios |
| Account labels, quota groups and lease join | [`metaproc/src/metaproc/dispatch/credential_pool.py:160-214`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/dispatch/credential_pool.py#L160-L214); [`metaproc/src/metaproc/runpool/event_models.py:136-195`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/runpool/event_models.py#L136-L195) | Share format | New metaproc source bead; `uro-eamm` | Labels are aliases; `account_id` may hash a rotating token; never read pool files |
| Quota from OAuth endpoint and error text | [`metaproc/src/metaproc/adapters/claude_code.py:1314-1404`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/adapters/claude_code.py#L1314-L1404); [`metaproc/src/metaproc/agent_errors.py:26-126`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/agent_errors.py#L26-L126) | Learn only | `uro-eamm` | Network access, credentials and text inference conflict with recorded-window accounting |
| Unknown-as-zero defects | [`metaproc/src/metaproc/engine/resource_rollup.py:800-816`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/engine/resource_rollup.py#L800-L816); [`metaproc/src/metaproc/trace/aggregation.py:103-136`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/trace/aggregation.py#L103-L136); [`metaproc/docs/arch/arch-file-io-utilities.md:118-125`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-file-io-utilities.md#L118-L125) | Learn; port tests as negative cases | `uro-spce`, `uro-unib` | Each defect becomes a golden case |
| Tailer partial-line and truncation bugs | [`metaproc/src/metaproc/logutil/parsing.py:1681-1721`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/logutil/parsing.py#L1681-L1721) | Learn; port tests as negative cases | `uro-y2qj`; contracts: Source Snapshots | The pending-tail rule already addresses them |
| Memory accounting rules and footprint measurements | [`metaproc/docs/memory-accounting-reference.md:16-212`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/memory-accounting-reference.md#L16-L212); [`metaproc/docs/arch/arch-runpool.md:434-497`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/arch/arch-runpool.md#L434-L497) | Port document content | `uro-sbnk`; contracts: Entities (resource observation) | Use for benchmark memory and any future collector; RSS figures are floors |
| Performance lessons | [`metaproc/docs/performance-notes.md:21-142`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/performance-notes.md#L21-L142) | Learn | `uro-tzjq`, `uro-sbnk`, `uro-g8r0` | Cold and warm distributions, scoped discovery, one read, payload caps that report rather than drop |
| Verified publish sequence | [`metaproc/src/metaproc/commands/gzip_text.py:202-326`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/commands/gzip_text.py#L202-L326) | Port code | `uro-1k0u`, `uro-xm48`; contracts: Capture Cache | Use unique temp names and fsync the directory before replacing the manifest |
| Commit fencing | [`metaproc/docs/execution-model-design.md:156-182`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/docs/execution-model-design.md#L156-L182) | Learn | `uro-1k0u`, `uro-xm48` | A writer publishes only if the entry version it started from is still current |
| Locks and leases | [`metaproc/src/metaproc/io/mkdir_lock.py:125-183`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/mkdir_lock.py#L125-L183); [`metaproc/src/metaproc/io/orchestrator_lease.py:204-280`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/orchestrator_lease.py#L204-L280) | Learn only | `uro-1k0u`; contracts: Capture Cache | Age-based reclaim without an owner token is unsafe |
| Archive rejection rules | [`metaproc/src/metaproc/io/safe_archive.py:12-90`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/src/metaproc/io/safe_archive.py#L12-L90); [`metaproc/tests/io/test_safe_archive.py:37-85`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/tests/io/test_safe_archive.py#L37-L85) | Port code; port tests | `uro-unib`; contracts: Observation Bundles | Add decompressed-byte limits and backslash, drive-letter, case-fold and file-directory checks |
| Supply-chain checker | [`metaproc/devtools/check_supply_chain.py:12-124`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/devtools/check_supply_chain.py#L12-L124); [`metaproc/uv.toml:1-11`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/uv.toml#L1-L11); [`metaproc/.npmrc:1-19`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/.npmrc#L1-L19) | Port code | `uro-phi8`; baseline: Supply Chain | Add permission and waiver-expiry checks; exempt exact versions with review dates |
| Hygiene scanner and fixture generator | [`metaproc/devtools/public_hygiene.py:41-115`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/devtools/public_hygiene.py#L41-L115); [`metaproc/devtools/synthesize_fixtures.py:17-208`](https://github.com/jlevy/metaproc/blob/9b2e5ad51ab16f666f9478ba81b79e0988923d11/devtools/synthesize_fixtures.py#L17-L208) | Port code as Python dev tooling | `uro-obx5`, `uro-phi8` | Add datetime, UUID, provider-key and macOS temp-path patterns; remap IDs consistently to keep linkage |
| metaproc captures and signal fixtures | `metaproc/tests/fixtures/trace_agents/`, `metaproc/tests/fixtures/log_compaction/` and `metaproc/tests/fixtures/claude_api_signals/` | Reuse fixtures | `uro-obx5` | Scrub timestamps, session UUIDs, message and tool IDs, and signatures first |
| Claude repeated blocks and cumulative cost cases | [`qm/test/claude-harness-turn.test.ts:147-250`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/test/claude-harness-turn.test.ts#L147-L250); [`qm/src/harness/claude-harness.ts:531-608`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/claude-harness.ts#L531-L608) | Port tests | `uro-y2qj`, `uro-spce` | Synthetic SDK streams; select one record per request with a disagreement diagnostic rather than field-wise maximum |
| Codex running-total dedupe | [`qm/src/harness/codex-harness.ts:145-169`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/codex-harness.ts#L145-L169); [`qm/test/codex-harness.test.ts:233-244`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/test/codex-harness.test.ts#L233-L244) | Port code; port tests | `uro-y2qj`, `uro-spce` | Same policy as squares recommendation 2; totals are per thread, but a forked child starts from its parent’s total |
| Per-step timing with gap phases and residual | [`qm/src/sessions/session-store.ts:109-193`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/sessions/session-store.ts#L109-L193); [`qm/src/harness/pi-harness.ts:396-526`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/pi-harness.ts#L396-L526) | Share format; learn | contracts: Measure Contracts (Time); `uro-d135` | Explicit residual complements squares’ exclusive categories |
| Cache hit ratio with null semantics | [`qm/src/admin/metrics-sink.ts:41-62`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/admin/metrics-sink.ts#L41-L62) | Port code | `uro-d135`, `uro-tzjq` | Report the pooled ratio from token sums; never average per-turn ratios |
| Mixed `input` semantics and UI double count | [`qm/src/harness/codex-harness.ts:133-143`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/codex-harness.ts#L133-L143); [`qm/plugins/admin/public/index.html:13795-13824`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/plugins/admin/public/index.html#L13795-L13824) | Learn; port tests as negative cases | `uro-spce`, `uro-tzjq` | Normalize in adapters; the UI renders server values only |
| Estimate-driven budgets and uncounted side calls | [`qm/src/ratelimit/budget.ts:13-50`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/ratelimit/budget.ts#L13-L50); [`qm/src/harness/harness.ts:139-149`](https://github.com/yc-software/qm/blob/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76/src/harness/harness.ts#L139-L149) | Learn only | Plan: Non-Goals and Open Questions | Harness ledgers need an export contract before urollup can read them |

## Key Insights

1. **Harnesses hide usage from log readers, and sometimes duplicate it.** metaproc and
   qm turn off native persistence for Claude Code and Pi, qm also for Codex and
   OpenCode, and both delete per-attempt config directories.
   Their usage is invisible to default discovery, while a local metaproc Codex run has a
   capture and a rollout for one thread that share only the thread ID, not a request
   key.
2. **A stream’s totals can exceed, or repeat, its itemized requests.** `claude-stream`
   `modelUsage` names models, such as Haiku, that may have no assistant records;
   top-level `usage` covers only the main model; and `total_cost_usd` is cumulative
   across several results in a streaming-input session.
   Treating result totals purely as checks either raises a permanent mismatch or drops
   real usage, so the residual needs its own measure.
3. **Token field names do not carry their inclusion rules.** qm’s shared `input` field
   includes cached tokens for Codex and excludes them for the others, which
   double-counted cache in its UI and broke its hit ratio.
   Normalization belongs in each adapter, with the native value kept.
4. **Captured streams lack timestamps where accounting needs them.** `claude-stream`
   assistant and limit records, every `codex-exec` record and Pi tool events have none,
   so calendar buckets for captured streams rest on an inference rule; metaproc’s 1970
   sentinel shows what happens without one.
5. **`claude-stream` records provider limits too.** Its `rate_limit_event` carries
   window type, status, reset time and overage state, written on state changes and
   without timestamps, and neither metaproc nor qm reads Codex `rate_limits`.
6. **Harnesses rewrite logs after the fact.** Rewrite headers, dropped streaming events,
   synthetic `message_final` records, gzip replacement and a moment when the path is
   absent all change what urollup reads and how a snapshot or cache entry recognizes a
   source.
7. **Terminal summaries lose failed work, and copies look like requests.** Terminal-only
   tallies miss killed attempts, `codex-exec` never records usage for a failed or
   interrupted turn and reports a resumed thread’s earlier runs again, the Pi event
   stream repeats each message’s usage across five event types of which only
   `message_end` is final, and harness-seeded Pi history carries zero-usage assistant
   messages that are not requests.
8. **Unknown-as-zero recurs under an explicit rule.** metaproc stated the rule and still
   shipped null-skipping rollups, zero-filled aggregation, zero-omitting serialization,
   a `None`-suppressing YAML writer and an epoch-zero timestamp fallback; qm hard-codes
   zero cache writes and cost.
   Principles need a negative test for each path.
9. **Estimates and side calls distort budgets.** qm budgets on pre-call estimates and a
   flat input price, and neither harness accounts for every auxiliary call.
   urollup’s value-status labels prevent the first; the second means local totals from
   harness-driven work are partial by construction.
10. **Configured accounts can be wrong, limits span organizations, and fast mode is its
    own tier.** Ambient credentials override configured ones silently, `apiKeySource` is
    observed evidence, metaproc’s notes report Anthropic limits per organization, and qm
    shows fast mode billed and rate-limited separately.
11. **RSS is the wrong memory metric on macOS.** It omits compressed pages and
    overcounts shared pages, with peaks four times steady state in one measurement,
    which matters for urollup’s own benchmark gates as much as for any future collector.
12. **Tolerant readers hide snapshot problems.** Skipped malformed lines, replacement
    decoding, offsets that jump past partial lines, resets without identity checks and
    per-line caps that drop oversized records each turn a detectable condition into
    silent loss.

## Recommendations

Items marked *(Updated)* were revised on 2026-09-14 against Codex and Pi source; the
[squares review](research-2026-09-14-squares-code-review.md) recommends the rollout
parsing, replay, `token_count` and time-measure changes cited here by number.

### Plan Changes

- **P1 (Sources and snapshot boundary):** name harness run directories as a captured
  source layout: `.logs/tasks/**/*.jsonl` or `.jsonl.gz` with optional
  `<log>.invocation.json` sidecars.
  Accept gzip for every captured dialect, and report a leading metaproc `compaction`
  record as a transformed source in coverage.
- **P2 (Ledger, identities and accounting):** *(Updated: linkage and total semantics
  settled by source.)* Add a cross-dialect linkage rule.
  A `codex-exec` capture whose `thread.started.thread_id` equals a discovered rollout’s
  `session_meta.id` is a capture of that thread: rollout observations own the usage, and
  `turn.completed.usage` becomes a reconciliation check against the rollout’s cumulative
  thread total, excluding subagent usage and failed or interrupted turns.
  An unmatched capture contributes per-turn usage only as differences of consecutive
  totals for its thread, labeled with coverage gaps for failed and interrupted turns.
  A `claude-stream` capture and its transcript reconcile through `session_id` and
  `message.id`.
- **P3 (Relationships, in the plan and the contracts):** add `claude-stream` inline
  subagent messages as spawn edges keyed by `parent_tool_use_id`.
- **P4 (Usage windows):** list `claude-stream` `rate_limit_event` as a third recorded
  limit source, and state that utilization units differ (fraction or percent) and are
  kept natively with a unit.
- **P5 (Sources and snapshot boundary, accounts):** allow an optional organization or
  quota-group identifier per manifest account, record `apiKeySource` as an observed
  property, and diagnose conflicts with configured attribution; keep people (principals)
  distinct from billing accounts.
  Add both to Decisions to Confirm.
- **P6 (Non-Goals and Open Questions):** state that harnesses which discard native logs,
  such as qm, are out of scope until a tested export adapter exists, and add an open
  question on a harness-ledger import contract that records each row’s token inclusion
  rules.
- **P7 (Testing Strategy):** *(Updated: Codex and Pi cases.)* Add cases drawn from these
  bugs: a partial tail followed by an append; truncation then regrowth; a path absent
  during a rewrite; `.jsonl` replaced by `.jsonl.gz`; an attempt killed before its
  terminal record; `turn.failed` without usage; an interrupted `codex exec` turn with no
  terminal event; Pi usage repeated across stream event types in each `message_update`
  version range, and zero-usage seeded messages; cumulative `total_cost_usd` across
  several results; cumulative `turn.completed.usage` across a resumed thread’s captures;
  timestamp-less limit records from two sources in both orders; and error keywords
  inside tool output. CI must fail when a snapshot is missing.
- **P8 (Testing Strategy, benchmarks):** record macOS peak `phys_footprint` beside
  `getrusage` peak RSS, report cold and warm runs as distributions, and add a scale test
  that catches quadratic reconciliation.
- **P9 (Project setup):** *(Updated: the plan’s code reuse decision now covers the
  licensing half.)* Record the source repository and commit for every ported file, test
  or fixture in a provenance notice, add the license notice for third-party code such as
  qm, and scrub fixtures of timestamps, IDs and signatures before committing them.

### Data Contract Changes

- **C1 (Measure Contracts, counting rules):** compare source totals (`claude-stream`
  `modelUsage` per model, Pi `agent_end`) with reconciled requests and record any excess
  as an `unitemized` measure in coverage, reported beside totals and never inside them,
  with `--strict` exiting 3, because it has no request identity to merge on.
  Treat `total_cost_usd` across several results in one session as cumulative.
- **C2 (Measure Contracts, token categories):** require each adapter to derive disjoint
  uncached input, cache reads and cache writes, and keep the native inclusive value as a
  native field; name the tool measure a tool interval, never latency or execution time.
- **C3 (Time, Grouping and Percentiles):** add an inferred-timestamp rule for records
  without timestamps: use the nearest timestamped records before and after, else the
  sidecar `captured_at` and file modification time, labeled `inferred`, never a sentinel
  such as epoch zero.
- **C4 (Entities, provider limit observation):** add a `unit` field, keep overage fields
  verbatim, and record whether the observation time was recorded or inferred.
- **C5 (Entities, source-reported cost):** diagnose a zero source-reported cost on
  nonzero tokens, record when a source cost comes from a harness catalog, and never let
  a source estimate stand in for a list-price estimate.
- **C6 (Price Table):** list dated model IDs and context-variant suffixes as explicit
  aliases; match on `speed` for fast mode and on cache-write duration for one-hour
  writes; take the billing channel from recorded provider fields such as Pi’s
  `provider`; add repricing scenarios for a promotion whose end date moves, a mid-life
  cache-read cut and a 200K-token band; and test that the table review date is no older
  than any row’s.
- **C7 (Capture Cache):** use an OS advisory lock, create staged files owner-only, fsync
  each segment and the entry directory before replacing the manifest, verify a segment’s
  digest before publishing it, use unique temp names, and replace the manifest only if
  the version the writer started from is still current.
  Treat a path that vanishes mid-scan as a mid-scan change and a new `.gz` sibling as
  replacement of the logical source.
- **C8 (Observation Bundles):** *(Updated 2026-09-14: bundles are plain folders, so this
  applies to folder path, link and digest checks rather than archive entries.)* adopt
  metaproc’s archive rejection list and add limits on decompressed bytes, backslashes,
  drive letters, NUL bytes, case-fold duplicates and file-directory conflicts.
- **C9 (Contract Authoring and Rust Validation):** writers emit explicit nulls for
  unknown values and quote every timestamp.

### Bead Changes

- **`uro-y2qj`:** add `claude-stream` limit records and inline subagents (P3, P4), C3
  timestamps, gzip and rewrite-header input, P2 linkage, coverage gaps for `turn.failed`
  and interrupted `codex exec` turns, a detection buffer that replays sniffed lines, and
  the metaproc captures and qm stream cases.
- **`uro-obx5`:** port and scrub metaproc captures (P9), port the hygiene scanner with
  the missing patterns, and add torn-tail and rotation fixtures, which neither project
  has.
- **`uro-spce`:** *(Updated: exec captures and forked totals.)* Add a property test that
  a group with any unknown member is partial, golden cases for metaproc’s review defects
  and qm’s mixed `input` semantics, and a running-total dedupe rule shared by rollouts,
  app-server streams and `codex-exec` captures that subtracts a forked child’s inherited
  total.
- **`uro-wuby`:** apply C6.
- **`uro-eamm`:** group windows by a configured organization or quota group, keep
  unrecognized `rateLimitType` values, and use no endpoint or error-text windows.
- **`uro-qnok`:** *(Updated: Pi source review supersedes the dedupe detail.)* Follow the
  Pi dedupe and fork rules in the portable research brief; from these harnesses, add
  metaproc’s synthetic `message_final` as a partial revision like `message_update`,
  zero-usage seeded messages as non-requests, and C5.
- **`uro-unib`:** apply C8 and C9.
- **`uro-d135`:** port the cross-agent tool taxonomy with native names kept, report the
  pooled cache read share from token sums, and label tool intervals per C2.
- **`uro-tzjq`:** cap evidence payloads at the boundary with a byte count and an
  explicit truncation flag, format evidence lazily, and render server-computed cache
  ratios only.
- **`uro-phi8`:** add the provenance notice (P9), fail CI on missing snapshots, and
  extend the supply-chain validator to workflow permissions and waiver expiry.
- **`uro-sbnk`:** apply P8.
- **`uro-1k0u` and `uro-xm48`:** apply C7.
- **`uro-g8r0`:** measure the substring prefilter with type confirmation, and include a
  quadratic pre-0.84.0 Pi `message_update` log in the corpus.
- **New bead, Phase 2:** a metaproc run-directory source that reads captures with
  sidecars, recognizes transforms, and attributes accounts from `auth_lease_acquired`
  events.
- **New bead, Phase 3:** evaluate a harness-ledger import for qm-style
  `session_llm_requests` exports, blocked on P6.

## Next Steps

- [x] Verify whether `codex exec --json` `turn.completed.usage` is per turn or a running
  total for a resumed thread: Codex source shows a cumulative per-thread total (not yet
  observed in a running process).
- [ ] Verify with an unrewritten capture that includes a subagent and a web fetch
  whether `claude-stream` `modelUsage` reports usage that no assistant record itemizes.
- [ ] Record which `status` and `rateLimitType` values Claude Code writes in
  `rate_limit_event`, and whether `utilization` appears near a limit.
- [ ] Check whether `claude-stream` `uuid` values match transcript `uuid` values when
  persistence is on.
- [x] Verify whether a Codex app-server parent thread’s `tokenUsage.total` includes its
  child threads: in source it does not, but a forked child’s total starts from its
  parent’s.
- [ ] Review and apply the plan, contract and bead changes above.
- [ ] Optionally file metaproc beads for its bugs (tailer partial lines, rate-limit
  merge order and `blocked` check, null-skipping rollups, Pi zero cost, stale documents)
  and report qm’s Claude inclusion bug and UI double count upstream.

## Methodology

- **Code:** read metaproc’s adapters, parsers, extractors, usage, pricing, log rewrite
  (compaction), quota and resource modules, I/O utilities, devtools and workflows at
  `9b2e5ad`, and qm’s harness adapters, session store, orchestrator usage paths,
  budgets, metrics and model catalog at `78dd4cc`, with test names and selected tests.
- **Documents:** read metaproc’s memory accounting reference, performance notes,
  execution model design and implementation, process framework concepts, artifact
  catalog, Claude Code harness, runpool, cloud execution, testing and file I/O
  architecture, relevant sections of the core and authentication architecture, runbooks
  and specs; qm’s README, security policy and contribution guide.
- **Parallel passes:** read-only passes covered metaproc quotas and accounts, formats
  and rollups, filesystem and supply chain, and design documents, and qm harness capture
  and accounting. Their key claims were rechecked against source before inclusion.
- **Fixtures:** a script printed record types, key names and counters from committed
  metaproc fixtures; no content values were copied.
- **Limits:** no tests were run and no files in either project were changed.
  metaproc’s git history covers only the 142 commits after its extraction and qm’s
  snapshot is a shallow clone.
  Captured-stream observations come from a few rewritten captures of specific agent
  versions and can change between releases.
- **Source reconciliation:** on 2026-09-14, Codex and Pi format claims were rechecked
  against read-only source reviews of Codex at `6b9826e` and Pi at `d981de1`, and Claude
  claims against ccusage at `bd7f89b` and Anthropic’s session-report and receipts
  plugins at `f0dce59`; conclusions from source were not observed in running processes.

## References

- [metaproc at `9b2e5ad`](https://github.com/jlevy/metaproc/tree/9b2e5ad51ab16f666f9478ba81b79e0988923d11)
- [qm at `78dd4cc`](https://github.com/yc-software/qm/tree/78dd4cc3a7e1e6b1d538f3dfdd7ef127e567db76)
- [Codex at `6b9826e`](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340)
- [Pi at `d981de1`](https://github.com/earendil-works/pi/tree/d981de1229ef899957bbe968bc8dcda02a21f477)
- [squares review](research-2026-09-14-squares-code-review.md)
- [Portable research brief](research-2026-09-13-portable-agent-usage.md)
- [Rust CLI engineering baseline](research-2026-09-13-rust-cli-engineering-baseline.md)
- [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md)
- [urollup design](../../urollup-design.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
