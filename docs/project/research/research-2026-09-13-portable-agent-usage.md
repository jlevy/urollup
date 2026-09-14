---
title: Portable Agent Usage Analytics
description: Public-source background for urollup, a Rust CLI and local read-only web UI that produces mergeable token, cost and usage rollups from Claude Code, Codex and Pi session logs.
date: 2026-09-13
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete for initial design and feeds the urollup plan; unverified dialect details and cloud export compatibility still need verification
---
# Research: Portable Agent Usage Analytics

## Overview

urollup is a planned Rust CLI and local read-only web UI that produces token, cost and
usage rollups from Claude Code, Codex and Pi session logs.
Agent logs already record token usage, so these rollups need no agent instrumentation
and no transcript upload to an observability service.
The product combines usage reports, request-size analysis and investigation that links
each number to its source records.
It must also combine local histories with results exported from cloud sandboxes without
counting copied work twice.

This brief collects the public-source background for that design: existing tools, what
each agent’s logs record, how to count each request once, and what an exported result
must keep so it can be merged later.
It feeds the [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md),
which turns these findings into contracts and implementation phases.
Private session examples are excluded.

## Questions to Answer

1. Which capabilities of existing tools can urollup reuse or adapt?
2. What do Claude Code, Codex and Pi logs record about usage, session identity and
   subagents, and how can a process find the session it runs in?
3. Which accounting distinctions are necessary for trustworthy rollups?
4. What must an exported result retain so it can be merged later?
5. With Rust as the implementation language, how can the engine run fast without losing
   correctness or provenance, the link from each number back to its source records?

## Scope

A **session** is one top-level agent conversation.
A **thread** is one conversation with its own native ID inside a session: the main
thread, or a subagent thread linked to the thread that spawned it.
Codex records both thread and root-session IDs, Claude Code identifies a subagent by an
agent ID inside its parent’s session, and Pi has no built-in subagents.

Included:

- **Agents:** Claude Code, OpenAI Codex and Pi, the open-source `pi` terminal coding
  agent.
- **Dialects:** A **dialect** is one record format written by one agent.
  Each agent has a persistent log dialect, which it writes to disk during a session, and
  a captured stream dialect, which a caller saves from the agent’s JSON output.
  [Log Dialects and Session Linkage](#log-dialects-and-session-linkage) defines all six.
- **Analytics:** Usage by request, session, subagent tree, project, calendar period,
  usage window, model, effort and account.
  A **usage window** is a provider rate-limit period that the logs record.
- **Workflows:** The [common workflows](#common-workflows) that select which sessions a
  rollup covers.

Excluded:

- Other agents that ccusage reads, such as OpenCode, Gemini CLI and Amp, until tested
  fixtures exist
- Billing blocks inferred from activity rather than recorded by a provider
- Live session dashboards such as agentfdr’s Board, agent steering and agent launchers

## Findings

### Common Workflows

Every workflow selects sessions, reconciles their records, and writes a **usage
summary**: a softschema YAML artifact with one shape for a single session or an
aggregate, described under [Portable Result Merging](#portable-result-merging).
A workflow writes either one summary for the whole selection or one per session.
**Reconciliation** groups every record of one provider request, across content blocks,
files and copied history, so its usage counts once.

1. **Current-session summary.** The most common case: an agent, hook or person inside a
   running Claude Code or Codex session asks for that session’s usage so far, including
   its subagents. urollup must find the session without arguments.
   Claude Code, Codex and current Pi releases all pass the session identity to the
   processes their tools spawn, so detection can rely on recorded signals and use
   working-directory heuristics only on request (see
   [Current-Session Signals](#current-session-signals)).
2. **Selected multi-session rollup.** A person or workflow selects sessions by agent,
   date range, explicit session IDs or transcript paths, project or working directory,
   and source root, then reads per-session summaries, one aggregate, or calendar
   breakdowns. Sessions span midnight and resume days later, so a date selection must
   state whether it clips usage to the interval or includes every session that touches
   it.
3. **Disk-wide inventory.** The broadest selection discovers every supported log under
   the default and configured roots, across all dates, and reports all usage.
   It doubles as a coverage check: which roots exist, which files are unsupported or
   truncated, and which subagent threads have no discoverable parent.
4. **Session hierarchy.** A crawler links each thread to the subagent threads it
   spawned, recursively, and each session to the session it was forked from.
   The resulting in-memory tree lets a rollup include a session’s descendants or only
   the session itself, and lets a summary show a thread’s own usage beside its
   subagents’ usage. Linkage differs per agent (see [Session Linkage](#session-linkage)).
   Codex writes each thread to its own log file, called a **rollout**, under a date
   directory, and a subagent’s rollout can start in a later date directory than its
   parent’s. The crawler therefore indexes session headers under every root rather than
   searching near the parent.

Claude Code and Codex **hooks** are user-configured commands that the agent runs at
lifecycle events, passing them JSON input.

The ways to identify sessions trade precision for reach:

| Selector | Precision | Failure modes |
| --- | --- | --- |
| Environment variable set by the agent | Exact session or thread ID in every tool subprocess | Inherited by nested agents and background launchers; absent in plain terminals and older releases |
| Hook input (`session_id`, `transcript_path`) | Exact transcript path, plus the subagent transcript on stop events | Only inside hooks; the transcript can lag the in-memory turn |
| Most recent transcript for the working directory | Works in any terminal and release | Wrong with concurrent sessions in one repository; worktrees and directory changes move Claude and Pi project directories; resumes and forks copy history into other files; cloud sandboxes may not expose logs |
| Explicit session IDs or transcript paths | Reproducible and scriptable | The caller must find the ID |
| Agent, date, project and root filters | Broad rollups without IDs | Usage timestamps decide membership, so a session can lie partly inside a range |

### Existing Implementations

[agentfdr](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90)
provides CLI reports, JSON summaries, search, comparisons, configurable anomaly
heuristics and a local session viewer.
Its 0.8.0 source normalizes Claude Code and Codex logs into a shared turn
representation. The Board, a separate Claude-specific system that joins live-process and
desktop data, is unnecessary for retrospective analysis.
The
[parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js)
groups repeated Claude message blocks, and the
[cost module](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/cost.js)
uses bundled pattern-matched prices with simplified cache multipliers.
Its summaries are useful evidence, not an authoritative provider billing ledger.

[ccusage](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1)
supplies the usage-reporting model: calendar and session rollups, token, cache and model
breakdowns, machine-readable output and price estimates.
Its 20.0.20 features are a baseline to measure against, not an implementation to
duplicate.
A parity matrix should name the exact release, command, dialect and accounting
scope, and prices missing offline must count as incomplete coverage.

The
[Anthropic session-report plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report)
wraps a deterministic analysis CLI in a reporting skill, and its analyzer attributes
usage to prompts, projects, days and subagents.
Its request-key fallbacks and summed file durations show why metrics must state their
definitions: an assistant record is not always a provider request, and the sum of file
spans is not elapsed time.

[Metabrowser’s log adapters](https://github.com/jlevy/metabrowser/blob/37011c447cc4cad2d684045683648f641c172144/src/metabrowser/logutil/parsing.py)
offer a streaming parser factory, dialect detection and evidence views.
Their display events can repeat one raw usage record across several content blocks, and
preview limits can truncate or omit large records.
urollup can reuse their format knowledge and tests, but accounting must read source
records rather than sum display events.
Pi’s event stream and its persistent branched sessions need separate fixtures, as do
Codex’s `exec` output and its rollouts.

### Log Dialects and Session Linkage

Field names below come from pinned public source, vendor documentation and key-name
inspection of local logs (see [Methodology](#methodology)); no values, paths or IDs were
copied. Claude Code calls its transcript entry format internal and subject to change
between releases
([sessions](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)), so
each adapter must record the agent versions its fixtures cover.

**Pi** is built on the multi-provider `pi-ai` library.
It was published as `@mariozechner/pi-coding-agent` from `badlogic/pi-mono`, the package
installed locally, and is now developed as
[earendil-works/pi](https://github.com/earendil-works/pi).
Metabrowser’s `PiLogParser` reads its `pi --mode json` event stream, and ccusage reads
its session files as the “pi-agent” source
([guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/pi/index.md)).
Pi calls many providers, so provider and model come from each assistant message, never
from the agent name.

| Dialect | Writer | Location | Records |
| --- | --- | --- | --- |
| `claude-project` | Claude Code sessions with persistence | `<config>/projects/<project>/<session-id>.jsonl`, `<project>` being the cwd with non-alphanumerics replaced by `-`; subagents in `<session-id>/subagents/agent-<agent-id>.jsonl` beside `agent-<agent-id>.meta.json` | One entry per message, content block or metadata record, with `type`, `uuid`, `parentUuid`, `sessionId`, `timestamp`, `cwd`, `gitBranch`, `version` and `isSidechain` |
| `claude-stream` | `claude -p --output-format stream-json --verbose`, saved by the caller | Explicit input only | SDK messages: `system` `init`; `assistant` and `user` with `session_id` and `parent_tool_use_id`; a final `result` with `usage`, `modelUsage`, `total_cost_usd`, `num_turns` and `duration_ms` |
| `codex-rollout` | Codex CLI, IDE extension and Desktop | `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<local-time>-<thread-id>.jsonl` and `archived_sessions/`; optionally `.jsonl.zst`; a thread revert writes a new file with a `_<rollout-id>` suffix | `{timestamp, ordinal, type, payload}` lines, `ordinal` only in paginated history: `session_meta`, `turn_context`, `response_item`, `event_msg`, `compacted`, `token_usage_record` and others |
| `codex-exec` | `codex exec --json`, saved by the caller | Explicit input only | `thread.started` (`thread_id`), `turn.started`, `item.started`, `item.updated`, `item.completed`, `turn.completed` (`usage`), `turn.failed` and `error`; no timestamps, model or response IDs |
| `pi-session` | Pi sessions with persistence | `$PI_CODING_AGENT_DIR/sessions/--<cwd>--/<timestamp>_<uuid>.jsonl`, default `~/.pi/agent/sessions`, unless `PI_CODING_AGENT_SESSION_DIR` or `--session-dir` relocates it | A `session` header (`version`, `id`, `cwd`, optional `parentSession`), then tree entries with `id` and `parentId`: `message`, `model_change`, `thinking_level_change`, `compaction`, `branch_summary`, `custom` and others |
| `pi-events` | `pi --mode json`, saved by the caller | Explicit input only | The session header, then `agent_start`, `turn_start`, `message_start`, `message_update`, `message_end`, `tool_execution_*`, `turn_end` and `agent_end` events |

Sources: Claude Code [directory](https://code.claude.com/docs/en/claude-directory),
[subagent](https://code.claude.com/docs/en/sub-agents) and
[SDK message](https://code.claude.com/docs/en/agent-sdk/typescript) docs; Codex
`rust-v0.154.0`
[recorder](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs),
[compression](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs),
[rollout line](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs)
and
[exec events](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs);
Pi v0.85.1
[session format](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/session-format.md)
and
[JSON mode](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/json.md).

#### Usage Fields

| Usage | `claude-project` | `codex-rollout` | `pi-session` |
| --- | --- | --- | --- |
| Request identity | `requestId` and `message.id` on each assistant entry | `token_usage_record` payload: `response_id`, `thread_id`, `turn_id`, `root_turn_id`, `session_id`; older rollouts have only `token_count` events, with no response ID | Entry `id`; optional `message.responseId` |
| Fields | `message.usage`: `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `output_tokens`, `cache_creation.ephemeral_5m_input_tokens` and `ephemeral_1h_input_tokens`, `output_tokens_details.thinking_tokens`, `server_tool_use`, `service_tier`, `speed`, `inference_geo`, `iterations` | `input_tokens`, `cached_input_tokens`, `cache_write_input_tokens`, `output_tokens`, `reasoning_output_tokens`, `total_tokens` | `message.usage`: `input`, `output`, `cacheRead`, `cacheWrite`, optional `cacheWrite1h` and `reasoning`, `totalTokens`, and a `cost` breakdown computed by Pi |
| Inclusion | `input_tokens` excludes cache reads and writes; top-level counts sum server-side `iterations` | `input_tokens` includes `cached_input_tokens`; `reasoning_output_tokens` is part of `output_tokens` | `input` excludes `cacheRead` and `cacheWrite`; `reasoning` is part of `output` |
| Counter kind | Per response, repeated on every content-block entry of one request | `token_count.info.total_token_usage` is cumulative per thread and `last_token_usage` the latest delta; `token_usage_record` holds per-response `usage` with turn and thread totals | Per assistant message |
| Model and effort | `message.model`; entry-level `effort` | `turn_context` `model` and `effort` per turn; `thread_settings_applied` service-tier changes | `message.provider`, `message.model`, optional `responseModel` and `providerThinkingLevel`; `model_change` and `thinking_level_change` entries |
| Provider windows | `quotaLimits` on some assistant entries: `rateLimitType`, `resetsAt`, `status` | `token_count.rate_limits.primary` and `secondary`: `used_percent`, `window_minutes`, `resets_at` | None recorded |

Evidence and caveats for the usage fields:

- **Claude:** Anthropic defines `input_tokens` as tokens after the last cache breakpoint
  ([prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)).
  In every local request group, all entries sharing a `requestId` had one `message.id`
  and identical usage.
  ccusage 20.0.20 nonetheless keeps the larger total when entries share a deduplication
  key
  ([lib.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs)),
  which suggests some versions record differing usage within a group (unverified).
  agentfdr takes context size from the last `iterations` element and output from the top
  level
  ([parser.js](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js)).
  `effort`, `iterations`, `speed`, `inference_geo` and `quotaLimits` were observed
  locally and are undocumented.
- **Codex:** The protocol defines `TokenUsage`, `TokenUsageRecord`, `SessionMeta` and
  `RateLimitWindow`
  ([protocol.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs)).
  agentfdr reports that `total_tokens` equals input plus output and that cached input
  never exceeds input
  ([codex.js](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js)).
  Local `last_token_usage` records agreed: cached input never exceeded input, reasoning
  never exceeded output, and about 99% had `total_tokens` equal to input plus output.
  Identical consecutive cumulative snapshots were common, and one cumulative total
  decreased, so a delta is valid only within a **counter epoch**, a span in which the
  cumulative total does not reset.
  `token_usage_record` appears in the `rust-v0.154.0` protocol and local 0.15x rollouts;
  its first release is unverified.
  Rollouts before 2025-09-06 contain no `token_count` events, and subagent rollouts that
  inherit parent context replay the parent’s history prefix, which ccusage excludes
  ([Codex guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/codex/index.md)).
  `codex-exec` reports only per-turn `usage`.
- **Pi:** The v0.85.1
  [message types](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/types.ts)
  add `responseId`, `responseModel`, `providerThinkingLevel`, `cacheWrite1h` and
  `reasoning`, which the 0.62.0 session docs lack.
  In every local assistant message, `totalTokens` equaled the sum of input, output,
  cache reads and cache writes.
  The locally inspected sample is small, and Pi provider adapters may differ.

#### Session Linkage

| Linkage | `claude-project` | `codex-rollout` | `pi-session` |
| --- | --- | --- | --- |
| Session identity | File name stem; entries copied by resume or fork keep their original `sessionId`, so one file can hold several | `session_meta.payload.id` is the thread; `session_id` is the root thread’s ID | Header `id` |
| Subagents | Entries carry `agentId` and `isSidechain: true`; `.meta.json` has `agentType`, `description`, `toolUseId` and `spawnDepth`; `toolUseId` matches the spawning `Agent` `tool_use.id` in the parent or another subagent, whose `toolUseResult` records `agentId` | `parent_thread_id`; `source.subagent.thread_spawn` with `parent_thread_id`, `depth`, `agent_nickname`, `agent_role` and `agent_path`; other sources `review`, `compact`, `memory_consolidation` and `other`; `thread_source` is `user`, `subagent`, `guardian_review`, `memory_consolidation` or a feature name | None built in; extensions such as Pi’s subagent example start separate `pi` processes with no documented linkage |
| Forks and resumes | `/branch` and `--fork-session` create a new session ID holding copied history; `--resume` keeps the ID; older transcripts keep subagent turns inline, marked `isSidechain`, with no spawn ID | `forked_from_id` with `forked_from_ordinal_exclusive`; `subagent_history_start_ordinal` marks where a subagent’s own records begin | Header `parentSession` path for `/fork`, `/clone` and `--fork`; in-file branches through `parentId` and `branch_summary.fromId` |

Claude Code documents the subagent directory and resume behavior
([subagents](https://code.claude.com/docs/en/sub-agents),
[sessions](https://code.claude.com/docs/en/sessions)). The `.meta.json` keys, copied
`sessionId` values and inline subagent turns come from local inspection and agentfdr’s
[subagents.js](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js).
In one inspected local session, each `.meta.json` `toolUseId` matched an `Agent` call in
the parent, and subagent entries’ `sessionId` equaled the parent session’s ID. agentfdr
also reads `subagents/workflows/<workflow-id>/agent-<agent-id>.jsonl`, which the
inspected logs did not contain.
In recent local Codex rollouts, every `thread_spawn` source named the same parent as
`parent_thread_id`, and `session_id` equaled `id` only for user threads.
Whether copied Pi fork entries keep their entry IDs is unverified.

#### Current-Session Signals

| Signal | Claude Code | Codex | Pi |
| --- | --- | --- | --- |
| Tool-subprocess environment | `CLAUDE_CODE_SESSION_ID` in Bash, PowerShell and hook subprocesses, updated on `/clear`; `CLAUDECODE` and `CLAUDE_CODE_CHILD_SESSION` mark nesting; `CLAUDE_PID` (v2.1.214+) is the agent process | `CODEX_THREAD_ID` (present by `rust-v0.100.0`); `CODEX_SESSION_ID`, the root session (added after `rust-v0.135.0`); `CODEX_VERSION` | `PI_SESSION_ID` and `PI_SESSION_FILE` (0.82.0+), with `PI_PROVIDER`, `PI_MODEL` and `PI_REASONING_LEVEL`; `AI_AGENT=pi` marks the process |
| Hook input | `session_id`, `transcript_path`, `cwd`; `SubagentStart` and `SubagentStop` add `agent_id` and `agent_type`, and `SubagentStop` adds `agent_transcript_path` | `session_id`, `turn_id`, `transcript_path`, `cwd`, `model`, `agent_id`, `agent_type`; `SubagentStop` passes the parent transcript plus `agent_transcript_path` | Extension API only |
| ID to transcript | `<config>/projects/*/<id>.jsonl`; `--resume <id>` searches the current project and its worktrees first | File name ending `-<thread-id>.jsonl` or `.jsonl.zst` under `sessions/` or `archived_sessions/` | `PI_SESSION_FILE` is the path |
| Cloud | `CLAUDE_CODE_REMOTE` and `CLAUDE_CODE_REMOTE_SESSION_ID`; whether that ID names a transcript file is unverified | Not surveyed | Not applicable |

Sources: Claude Code [environment variables](https://code.claude.com/docs/en/env-vars)
and [hooks](https://code.claude.com/docs/en/hooks); Codex
[shell_environment.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/shell_environment.rs),
[exec_env.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs),
[hooks schema](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs)
and
[hook runtime](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs),
with the version bounds checked at the `rust-v0.100.0` and `rust-v0.135.0` tags; Pi
[environment variables](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/environment-variables.md)
and
[changelog](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md).

What was verified in a running process, and what was not:

- Inside this research session’s Claude Code tool processes, `CLAUDE_CODE_SESSION_ID`
  named exactly one `<project>/<id>.jsonl` transcript under the encoded cwd, and
  `CLAUDE_PID` was an ancestor process.
  A subagent’s tool processes received the parent session’s ID, not an agent ID, so a
  Claude subagent cannot identify its own transcript from the environment.
  `AI_AGENT` was also set there, and Claude Code keeps an undocumented live registry,
  `~/.claude/sessions/<pid>.json`, with `pid`, `sessionId`, `cwd` and `status` keys.
- `CODEX_THREAD_ID`, `CODEX_SESSION_ID` and `PI_SESSION_ID` were verified from source
  and documentation only.
  Whether a Codex subagent thread’s tools receive the subagent’s thread ID or the
  parent’s is unverified.
- Claude Code documents that an MCP server subprocess keeps the ID it started with, and
  after `--continue` or `--resume` without an ID it may see the initial startup ID.
- Environment markers are inherited.
  Codex passes its full parent environment to tools by default, and Pi spreads
  `process.env`, so a Codex or Pi session started from Claude Code’s Bash tool exposes
  both agents’ variables.
  Detection must treat several agents’ variables as ambiguous rather than picking one.

### Meaning of Resource Usage

A model request, an assistant message, a tool action, a nested local command, a retry
and a session are different entities.
One request can produce several messages or tool calls, and a tool can invoke further
models, so a rollup records links only where the log proves them and never treats one
count as another.

Input tokens fall into uncached, cache-read and cache-write categories, and each
provider decides which categories its input count includes (see
[Usage Fields](#usage-fields)). Reasoning tokens may be a subset of output.
Repeated context can accumulate large totals without containing that many distinct
tokens, so request-size histograms and outliers are useful even when the components of a
request’s context cannot be attributed exactly.
Byte counts and estimated tokenization must stay separate from measured provider usage,
and visible reasoning text is not a complete record of internal reasoning.

Five kinds of cost stay distinct: list-price estimates, cost estimates reported by a
source, actual charges, subscription allocations and local-compute estimates.
A price needs an effective date, model and service-tier identity, currency,
cache-duration semantics and explicit missing coverage.
The public price datasets that ccusage embeds, LiteLLM’s
[model price file](https://github.com/BerriAI/litellm/blob/1a183efaa1a2108aed7e1bed8d445d93bd1aa60d/model_prices_and_context_window.json)
and
[models.dev](https://github.com/anomalyco/models.dev/tree/bff41227803631c84903fcf7f486370e9fbcde86),
record current rates without effective dates, so they cannot reprice historical usage on
their own. Unknown cost is not zero, and tokens, dollars, wall time and CPU time are
separate units.

A thread’s origin, purpose, execution environment and relationships to other threads are
independent properties.
An automatic permission review may be a separate thread in one source and an event in
another, so adapters preserve each agent’s native representation and evidence rather
than imposing one agent’s thread model on every agent.
Model and effort belong to each request when recorded, and unknown values stay visible
in rollups.

### Synthetic Double-Counting Example

All values in this example are synthetic.
They show record shapes that the cited parsers handle, not measurements from any
session.

**Claude Code: content blocks and resumed history.** Claude Code splits one API response
into several `assistant` JSONL records, one per content block.
The records share `message.id` and `requestId`, but only the last one carries the final
`output_tokens`. A resumed session can also re-serialize earlier records, with their
original `uuid`, into a new file.
Anthropic’s
[session-report analyzer](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L14-L27)
documents both behaviors, deduplicates by request ID and by `uuid`, and cites 3–10x
overcounting without the request-level step.
The agentfdr
[parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L185-L200)
likewise overwrites a turn’s usage instead of accumulating it.
When one record holds several blocks, the
[Metabrowser Claude adapter](https://github.com/jlevy/metabrowser/blob/37011c447cc4cad2d684045683648f641c172144/src/metabrowser/logutil/parsing.py#L329-L410)
emits one display event per block, and every event keeps the whole raw record, including
its usage.

Suppose one response has thinking, text and tool-use blocks, and a resumed session file
replays its last record:

| Record | File | `uuid` | `message.id` | `requestId` | Block | Input | Cache read | Output |
| --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |
| 1 | `s1.jsonl` | `u1` | `msg_A` | `req_A` | thinking | 3 | 40,000 | 12 |
| 2 | `s1.jsonl` | `u2` | `msg_A` | `req_A` | text | 3 | 40,000 | 12 |
| 3 | `s1.jsonl` | `u3` | `msg_A` | `req_A` | tool_use | 3 | 40,000 | 600 |
| 4 | `s2.jsonl` (resumed) | `u3` | `msg_A` | `req_A` | tool_use | 3 | 40,000 | 600 |
| Naive sum: 4 requests |  |  |  |  |  | 12 | 160,000 | 1,224 |
| Reconciled: 1 request |  |  | `msg_A` | `req_A` |  | 3 | 40,000 | 600 |

The reconciled request uses the final output count and keeps all four records as
evidence references.
ccusage 20.0.20 deduplicates Claude entries by message ID and request ID across every
scanned file, falls back to the message ID alone when a log marked `isSidechain` replays
parent messages under new request IDs, and keeps a non-sidechain duplicate over a
sidechain one, then the duplicate with the largest token total
([Claude adapter](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L126-L200)).
Such keys are policy choices, not universal identities: a later ccusage commit adds the
session ID to its key, with a
[test](https://github.com/ccusage/ccusage/blob/1b4f42314bf9fe2f323436d2dadba18ba9b04970/rust/adapters/claude/src/lib.rs#L748-L775)
for gateways that reuse message IDs across distinct sessions.

**Codex: cumulative counters and fork replay.** Codex `token_count` events carry
`total_token_usage`, the running thread total, and `last_token_usage`, the latest delta.
ccusage guards against an event that repeats without the running total advancing:

| Event | `total_token_usage` total | `last_token_usage` total |
| --- | ---: | ---: |
| 1 | 10,000 | 10,000 |
| 2 | 25,000 | 15,000 |
| 3 (repeat) | 25,000 | 15,000 |

Summing the running totals gives 60,000 tokens, and summing every `last_token_usage`
gives 40,000. The reconciled total is 25,000. The ccusage
[Codex parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L317-L335)
uses `last_token_usage` only when the running total advances and otherwise subtracts the
previous total, so event 3 contributes nothing.
Its
[replay plan](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs)
also skips the parent token history that forked and spawned child rollouts replay.

### Portable Result Merging

Session IDs alone cannot make merging safe.
Two exports can contain different portions of one session, overlapping response
histories, or a later correction of earlier usage.
Summing both totals double-counts the overlap, and dropping one session loses new work.

The general solution is to export **observations**. An observation is a compact record
of one provider request or response as one source recorded it: identity, usage revision,
owning thread, source scope, model, effort, token usage and provenance.
A **usage revision** distinguishes successive usage values recorded for one request,
such as a streamed update and its final count.
Tool and resource facts are added when an analysis needs them.
Merging needs no prompt text, tool arguments or result bodies, so exports omit them.
An **observation bundle**, a versioned `*.urollup.zip` archive, carries observation
tables and always contains the usage summary computed from them.
Bundles merge by the same reconciliation rules as raw logs, and a merged bundle can be
merged again. Merge should be associative, commutative and idempotent for compatible
inputs, so grouping, order and repetition never change the result.
Conflicts are preserved deterministically, and missing identity or incompatible schemas
produce visible ambiguity or rejection, not invented certainty.

A totals-only report is useful for presentation but cannot generally be merged safely.
A summary becomes mergeable when it records what it covers.
Call each covered unit an **extent**: the requests of one owner thread that one export
covers, recorded as a digest of their request IDs and usage revisions and, optionally,
the IDs themselves.
With extents, a merge can tell whether two are identical, whether one
covers the other, or whether they are disjoint, and it adds usage only in those cases.
Keeping only the extents that no other extent covers makes merge the join of a
**semilattice**, an associative, commutative and idempotent operation.
That property lets state-based
[conflict-free replicated data types](https://inria.hal.science/inria-00609399) converge
regardless of delivery order or repetition.
Extents that overlap without containment, or that disagree on a usage revision, cannot
be combined from totals and need observations.

Non-additive statistics need mergeable forms.
Count, sum, minimum and maximum merge exactly, and histograms with fixed logarithmic
buckets merge by addition with bounded relative quantile error, as in
[DDSketch](https://arxiv.org/abs/1908.10693). Averaging group percentiles is wrong.
One summary format can therefore describe a single session or an aggregate, with
observation bundles as the exact fallback.

[softschema](https://github.com/jlevy/softschema/tree/ff0f91999ebd0c3e8e0864e76042c6d875be083e),
the same maintainer’s validator for YAML and Markdown artifacts with gradual contracts,
fits the summary contract.
Each artifact is a YAML payload that names its contract and a maturity status.
softschema compiles a Pydantic model into a deterministic JSON Schema Draft 2020-12
document identified by `schema_sha256`, which a Rust binary can validate against without
Python
([spec](https://github.com/jlevy/softschema/blob/ff0f91999ebd0c3e8e0864e76042c6d875be083e/docs/softschema-spec.md#compiled-schemas)),
and `compile --check` catches drift between a model and its committed schema.
Gradual contracts suit measures that stabilize one dialect at a time: a value can stay
in an open extension map until a consumer relies on it.
Three limits shape the design:

- softschema defines no JSON or JSONL artifact profile, so high-volume observation
  tables are validated row by row outside the softschema CLI.
- Its `enforced` status closes objects inside softschema’s own validator without
  changing the compiled schema, so models must declare closed objects for another
  validator to agree.
- Pydantic validators are not part of the portable schema, so cross-field invariants
  belong in the Rust core.

Account display aliases are unstable and must not serve as identity namespaces.
Exports should carry stable source-account identifiers, or explicit alias mappings
across exports, when available; account equality is never inferred from similar session
activity.

A skill running in Claude Code Cloud can invoke a compatible installed CLI on the logs
the sandbox exposes, write a bundle to a supported artifact location, and make that
artifact available for download.
Local analysis then merges it with local logs or other bundles.
Access to complete cloud logs, installation and network permissions, and artifact
retrieval vary by environment and need an actual cloud smoke test.
The design supports this workflow, but this research does not show that every cloud
environment exposes every required log field.

### Rust as a Constraint

Rust is a decided constraint for urollup, not an option this brief evaluates.
It was chosen for four reasons:

- **One native binary:** A single executable with embedded web assets runs the same way
  on a laptop and in a cloud sandbox, with no language runtime or package environment to
  install first.
- **Streaming performance:** Buffered reads, compact typed records and bounded parallel
  parsing across files suit large log corpora, with explicit control over memory.
- **One typed accounting core:** The CLI and the HTTP API link the same library, so
  identities, reconciliation rules and report types cannot drift between surfaces.
- **Consistency with existing tools:** The author’s Rust CLIs
  [fdu](https://github.com/jlevy/fdu/tree/afbb2eef01e94f37a4462549b0828ca8337a5f4c), an
  incremental file roll-up engine, and
  [flowmark-rs](https://github.com/jlevy/flowmark-rs/tree/f1e9337e2d87ba614c231f0d17a2c181a3634117),
  a Markdown formatter, already ship prebuilt binaries, so urollup can follow their
  build, test and release conventions.

A TypeScript package, as ccusage was through 19.x, installs easily through npm and
shares a language with the browser UI, but it needs a Node runtime; the ccusage 20.0.20
npm package is itself a small Node launcher for a Cargo-built native executable.
Python, as in Metabrowser, would reuse existing adapter code and tests directly, but it
needs a Python environment in every sandbox, and CPU-bound record parsing is typically
slower. flowmark-rs reports more than 50x faster processing of large file sets than
Python Flowmark; that is one data point for text-processing workloads, not a prediction
for urollup.
Rust costs slower iteration, and format knowledge must be ported rather than
imported.

Apart from the embedded browser assets, all non-Rust code is development tooling.
A Python softschema toolchain authors and checks the summary contract, and the Rust code
consumes the compiled JSON Schema; no Python ships in or runs from the binary.
The
[Rust CLI engineering baseline](research-2026-09-13-rust-cli-engineering-baseline.md)
covers the rest of the development toolchain.

### Memory and Caching

Language choice alone does not guarantee speed.
Deduplication maps, group cardinality and exact percentiles can grow with the corpus
even when file reads stream, so the engine needs explicit memory budgets and an
ephemeral spill path for larger inputs.
That temporary working storage is not a persistent incremental cache.

Persistent caching should follow a correct uncached reference, but stable source and
observation identities are needed from the start.
A later cache must handle appends, partial lines, replacement, deletion, late
corrections, and changes to parser or pricing versions.
Re-importing identical data must never add usage, and cached and uncached reports must
agree for the same source snapshot and query.

## Options Considered

The approaches differ in architecture; any new code is Rust (see
[Rust as a Constraint](#rust-as-a-constraint)).

### Option A: Wrap Existing CLIs

**Description:** Run ccusage, agentfdr and similar tools and combine their reports.

**Pros:**

- Useful reports immediately

**Cons:**

- The tools’ identities, deduplication keys, scopes and pricing semantics differ and
  stay different

### Option B: Fork an Existing Viewer

**Description:** Start from an existing session viewer, such as agentfdr’s, and add
rollups.

**Pros:**

- Faster initial UI

**Cons:**

- Solves neither request accounting nor merging of exported results

### Option C: Shared Rust Engine with CLI and Web UI

**Description:** One Rust accounting and query core that both the CLI and the local web
UI call.

**Pros:**

- One accounting contract for every surface
- Portable output
- Native execution

**Cons:**

- Requires careful adapters, compatibility tests and ongoing maintenance

## Recommendations

Adopt Option C:

- Build a narrow shared engine, using existing tools as behavioral references and
  Metabrowser’s adapter boundary as a design reference.
- Support Claude Code and Codex first, then add Pi and imported cloud formats as each
  passes its own fixtures.
  Scope support by validated dialect and metric, not by vendor.
- Encode the [synthetic double-counting cases](#synthetic-double-counting-example) as
  golden fixtures before building reports on any adapter.
- Ship uncached CLI reports, usage summaries and observation bundles first, then the
  read-only web UI and workflow skill, then persistent caching.
- Compute every number in the shared query engine; the web UI renders those results and
  computes no authoritative totals of its own.

## Next Steps

This research is complete for initial design and feeds the
[urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md), which owns the
implementation phases.
These items still need verification:

- [ ] Resolve the unverified dialect details: whether some Claude Code versions record
  differing usage within one request group, the first Codex release with
  `token_usage_record`, whether a Codex subagent’s tools receive its own thread ID or
  its parent’s, whether copied Pi fork entries keep their entry IDs, and whether
  `CLAUDE_CODE_REMOTE_SESSION_ID` names a transcript file.
- [ ] Survey Codex signals in cloud environments, which this brief did not cover.
- [ ] Recheck locally observed behavior on more agent versions: the undocumented Claude
  Code fields (`effort`, `iterations`, `speed`, `inference_geo` and `quotaLimits`), the
  `subagents/workflows/` layout that agentfdr reads, and Pi’s `totalTokens` sum across
  provider adapters.
- [ ] Run a small cloud export, download and local-merge smoke test in each cloud
  environment, covering log visibility, installation and network access, artifact
  retrieval and overlapping re-export, before advertising its compatibility.
- [ ] Check that public fixtures are synthetic or sanitized, with no prompt, path,
  identity or credential leakage.

## Methodology

The findings come from public source at pinned revisions, vendor documentation, package
metadata on GitHub and the npm registry, and key-name inspection of local logs:

- **agentfdr 0.8.0, commit `e0904bf`:** The Claude and Codex parsers, the subagent
  reader, the cost module and the live Board.
- **ccusage 20.0.20, commit `bd7f89b` (tag `v20.0.20`):** The npm launcher, its six
  native platform packages, the Rust Claude and Codex adapters with their deduplication,
  cumulative-counter and fork-replay tests, and the Pi and Codex guides.
  The `v19.0.0` TypeScript source and the later main-branch commit `1b4f423` were read
  for comparison.
- **Anthropic session-report plugin, commit `f0dce59`:** The `analyze-sessions.mjs`
  analyzer and its notes on transcript structure.
- **Metabrowser, commit `37011c4`:** The log adapters in `logutil/parsing.py`.
- **Codex `rust-v0.154.0`, commit `6b9826e`:** The rollout recorder and compression,
  history, exec events, protocol, shell environment and hooks, with environment-variable
  version bounds checked at the `rust-v0.100.0` and `rust-v0.135.0` tags.
- **Pi v0.85.1, commit `d981de1`:** The session format, JSON mode and environment
  variable docs, the message types and the changelog.
- **Claude Code and Anthropic API documentation:** Retrieved 2026-09-13; these pages are
  not versioned.
- **Local logs:** Key names only, from Claude Code 2.1-series transcripts, Codex Desktop
  0.15x rollouts and Pi 0.62.0 sessions, plus environment checks inside this research
  session’s Claude Code tool processes.
  No values, paths or IDs were copied.
- **LiteLLM and models.dev price data:** The snapshots that ccusage 20.0.20 pins in its
  `flake.lock` (commits `1a183ef` and `bff4122`), checked for effective-date fields.
- **fdu and flowmark-rs:** READMEs and build metadata at commits `afbb2ee` and
  `f1e9337`, as context for the Rust constraint.
- **softschema 0.8.1, commit `ff0f919` (tag `v0.8.1`):** The README and specification,
  for the summary data contract.

The synthetic example was checked against the cited parser code and tests, not against
session logs or provider invoices.
Log behaviors come from parser code, tests, vendor documentation, the session-report
analyzer’s empirical notes and a small local sample, so they can change between agent
versions.
No benchmarks were run for this brief, and it makes no universal performance or
billing-accuracy claim.
Cloud acquisition and persistent caching are proposed designs, not tested integrations.

## References

Tools and implementations, at the inspected revisions:

- [agentfdr 0.8.0 source](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90):
  [Claude parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js),
  [Codex parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js),
  [subagent reader](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js)
  and
  [cost module](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/cost.js)
- [ccusage 20.0.20 source](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1):
  [Claude adapter](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs),
  [Codex parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs),
  [Codex replay plan](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs),
  [Codex guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/codex/index.md)
  and
  [Pi guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/pi/index.md)
- [ccusage 20.0.20 npm package](https://www.npmjs.com/package/ccusage/v/20.0.20)
- [ccusage 19.0.0 TypeScript source](https://github.com/ccusage/ccusage/tree/c5049cef6d830a6eef216534331ea3fa3da99314/apps/ccusage/src)
- [ccusage Claude adapter at commit `1b4f423`](https://github.com/ccusage/ccusage/blob/1b4f42314bf9fe2f323436d2dadba18ba9b04970/rust/adapters/claude/src/lib.rs)
- [Anthropic session-report plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report)
  (official plugin; its transcript notes are empirical, not a format specification):
  [analyzer](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs)
- [Metabrowser log adapters](https://github.com/jlevy/metabrowser/blob/37011c447cc4cad2d684045683648f641c172144/src/metabrowser/logutil/parsing.py)
- [fdu](https://github.com/jlevy/fdu/tree/afbb2eef01e94f37a4462549b0828ca8337a5f4c)
- [flowmark-rs](https://github.com/jlevy/flowmark-rs/tree/f1e9337e2d87ba614c231f0d17a2c181a3634117)
- [softschema 0.8.1](https://github.com/jlevy/softschema/tree/ff0f91999ebd0c3e8e0864e76042c6d875be083e):
  [specification](https://github.com/jlevy/softschema/blob/ff0f91999ebd0c3e8e0864e76042c6d875be083e/docs/softschema-spec.md)

Agent log formats:

- [Codex `rust-v0.154.0` source](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340):
  [rollout recorder](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs),
  [compression](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs),
  [rollout line](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs),
  [exec events](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs),
  [protocol](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs),
  [shell environment](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/shell_environment.rs),
  [exec environment](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs),
  [hooks schema](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs)
  and
  [hook runtime](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs)
- [Pi v0.85.1 source](https://github.com/earendil-works/pi/tree/d981de1229ef899957bbe968bc8dcda02a21f477):
  [session format](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/session-format.md),
  [JSON mode](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/json.md),
  [environment variables](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/environment-variables.md),
  [message types](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/types.ts)
  and
  [changelog](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md)
- Claude Code documentation (official docs, unversioned, retrieved 2026-09-13):
  [sessions](https://code.claude.com/docs/en/sessions),
  [`.claude` directory](https://code.claude.com/docs/en/claude-directory),
  [subagents](https://code.claude.com/docs/en/sub-agents),
  [Agent SDK TypeScript reference](https://code.claude.com/docs/en/agent-sdk/typescript),
  [environment variables](https://code.claude.com/docs/en/env-vars) and
  [hooks](https://code.claude.com/docs/en/hooks)
- [Anthropic prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
  (official docs, unversioned, retrieved 2026-09-13)

Price data:

- [LiteLLM model price file](https://github.com/BerriAI/litellm/blob/1a183efaa1a2108aed7e1bed8d445d93bd1aa60d/model_prices_and_context_window.json)
  (community-maintained dataset)
- [models.dev](https://github.com/anomalyco/models.dev/tree/bff41227803631c84903fcf7f486370e9fbcde86)
  (community-maintained dataset)

Papers:

- Shapiro, Preguiça, Baquero and Zawirski,
  [Conflict-free Replicated Data Types](https://inria.hal.science/inria-00609399) (Inria
  research report RR-7687, 2011; the peer-reviewed version appeared at SSS 2011)
- Masson, Rim and Lee,
  [DDSketch: A Fast and Fully-Mergeable Quantile Sketch with Relative-Error Guarantees](https://arxiv.org/abs/1908.10693)
  (PVLDB 12(12), 2019; peer-reviewed)

Related project documents:

- [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md)
- [Rust CLI engineering baseline](research-2026-09-13-rust-cli-engineering-baseline.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
