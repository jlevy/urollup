---
title: Portable Agent Usage Analytics
description: Public-source background for urollup, a Rust CLI and local read-only web UI that produces mergeable token, cost and usage rollups from Claude Code, Codex and Pi session logs.
date: 2026-09-13
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete for initial design and feeds the urollup design and plan; dialect facts and reusable code were checked against Codex, Pi, ccusage, agentfdr and Anthropic plugin source at pinned commits, while runtime behavior of source-derived facts and cloud export compatibility still need verification
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

This brief collects the public-source background for that design: existing tools and the
code and tests urollup can port from them, what each agent’s logs record, how to count
each request once, and what an exported result must keep so it can be merged later.
It feeds the [urollup design](../../urollup-design.md) and
[plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md), which turn these
findings into contracts and implementation phases.
The [agent tool source reviews](research-2026-09-14-agent-tool-source-reviews.md) keep
the detailed Codex, ccusage, Pi, agentfdr and Anthropic plugin evidence condensed here,
with the status of each review recommendation.
Private session examples are excluded.

## Questions to Answer

1. Which code, tests and capabilities of existing tools and agent sources can urollup
   reuse or adapt, and under which license terms?
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
   Codex writes each thread to its own log file, called a **rollout**, under a
   local-time date directory.
   A subagent’s rollout can start in a later date directory than its parent’s, one
   thread can span several rollouts, and archiving moves rollouts into a flat directory.
   The crawler therefore indexes session headers under every root rather than searching
   near the parent.

Claude Code and Codex **hooks** are user-configured commands that the agent runs at
lifecycle events, passing them JSON input.

The ways to identify sessions trade precision for reach:

| Selector | Precision | Failure modes |
| --- | --- | --- |
| Environment variable set by the agent | Exact session or thread ID in every tool subprocess | Inherited by nested agents and background launchers; absent in plain terminals, older releases, Codex hooks and Pi user shell commands |
| Hook input (`session_id`, `transcript_path`) | Exact transcript path, plus the subagent transcript on stop events | Only inside hooks; Codex sends the root session’s `session_id` even inside a subagent; the transcript can lag the in-memory turn |
| Most recent transcript for the working directory | Works in any terminal and release | Wrong with concurrent sessions in one repository; worktrees and directory changes move Claude and Pi project directories; resumes and forks copy history into other files; cloud sandboxes may not expose logs |
| Explicit session IDs or transcript paths | Reproducible and scriptable | The caller must find the ID |
| Agent, date, project and root filters | Broad rollups without IDs | Usage timestamps decide membership, so a session can lie partly inside a range |

### Existing Implementations

[ccusage](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1)
supplies the usage-reporting model: calendar and session rollups, token, cache and model
breakdowns, machine-readable output and price estimates.
Release 20.0.20 is a Rust workspace of 16 agent adapter crates plus core, CLI, terminal
and test-support crates, shipped through npm as a Node launcher for a native binary,
with about 600 tests on synthetic inputs.
It is a behavioral baseline and a source of portable code and tests (see
[Reusable Code and Tests](#reusable-code-and-tests)), but not an accounting authority:

- **No single reconciliation path:** `ccusage daily` uses a second Claude parser that
  also reads `progress` records and double-counted when a sidechain replay preceded its
  parent until commit
  [`a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84);
  Codex daily, session and `--since` runs deduplicate with different keys
  ([daily.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L396-L462),
  [aggregate.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L531-L554)).
- **Silently dropped records:** a Claude record loses its usage when its line contains a
  nested `"id":null` or a similar null field, for example inside tool input (confirmed
  with a synthetic probe)
  ([lib.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L455-L500)).
  Timestamps with other than 0 or 3 fractional digits are skipped for Claude and Pi and
  abort a Codex report
  ([date_utils.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L111-L161)).
- **Coverage gaps:** it reads none of `token_usage_record`,
  `subagent_history_start_ordinal`, `rate_limits`, `turn_context.effort` or
  `.jsonl.zst`, at 20.0.20 or at `main` commit `95bbc41`, and it double-counts the
  copied history of Pi forks until commit
  [`809eeb6`](https://github.com/ccusage/ccusage/commit/809eeb6d52a2c7d13b9c65e10d4106109247390c).
- **Pricing:** unless `--offline`, it fetches LiteLLM prices from the `main` branch at
  runtime; model matching is fuzzy, money is `f64`, LiteLLM long-context tiers apply per
  token category rather than per request, unpriced tokens appear as cost 0 in JSON, and
  an unrecorded Codex service tier follows the user’s current `config.toml`
  ([pricing.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L644-L680),
  [speed.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/speed.rs#L23-L61)).

A parity matrix should name the exact release, command path, dialect and accounting
scope, pin `--offline` and `--mode calculate`, and count prices missing offline as
incomplete coverage.
The [ccusage feature inventory](#ccusage-feature-inventory) lists those command paths
and the release’s full user-facing surface.

[agentfdr](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90)
provides CLI reports, JSON summaries, search, comparisons, configurable anomaly
heuristics and a local session viewer.
Its 0.8.0 JavaScript source normalizes Claude Code and Codex logs into a shared turn
representation. The Board, a separate Claude-specific system that joins live-process and
desktop data, is unnecessary for retrospective analysis.
Its accounting has gaps that a feature matrix must explain:

- The
  [parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L185-L200)
  takes a Claude turn’s input and cache counts from the last `iterations` element, so
  totals and cost under-report input when a request has several iterations.
- The
  [Codex parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js)
  adds usage for every repeated `token_count` event without checking that the running
  total advanced.
- Nothing is deduplicated across files, and the
  [usage view](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/usage.js#L45-L61)
  reads only main transcripts, omitting subagent files while counting resumed copies
  twice.
- The
  [cost module](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/cost.js)
  matches prices by regular expression with one 1.25x cache-write multiplier for every
  model.

Its portable parts are the
[anomaly detectors](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/detect.js),
with strict configuration validation, and
[subagent spawn placement](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js),
both with tests. Its cache-thrash check misfires on providers that never report caching,
and its reconstructed 5-hour windows and “billed” token metric duplicate measures
urollup excludes or splits by category, so none is worth copying.

The
[Anthropic session-report plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report)
wraps a deterministic analysis CLI in a reporting skill, and its
[analyzer](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L304-L321)
attributes usage to prompts, projects, days, skills and subagent types.
Its request key is `requestId`, else `message.id` only when it starts with `msg_0` and
is longer than 10 characters, else a per-record key, so a record without `requestId`
whose message ID lacks that prefix counts once per content block.
Within one file the record with the largest `output_tokens` wins, and across files the
first file processed wins.
A global first-seen `uuid` set removes replayed history, so attribution depends on
processing order: main transcripts precede subagent files, which sends fork-style
subagent replays to the parent, but main files follow directory-walk order, so a resumed
session’s replay goes to whichever file is walked first.
Its “wall clock” sums file spans, which is not elapsed time.

session-report and the sibling
[receipts plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts)
share one **skill model**: the CLI emits JSON, a fixed template renders it, and the
agent fills only short narrative slots.
receipts adds honesty rules that fit urollup’s reporting skill: names from logs are
inert data, each table states which columns add up, unknown is not zero, estimates carry
no unlabeled dollar figures, and nothing is published by default
([SKILL.md](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L139-L164)).

[Metabrowser’s log adapters](https://github.com/jlevy/metabrowser/blob/37011c447cc4cad2d684045683648f641c172144/src/metabrowser/logutil/parsing.py)
offer a streaming parser factory, dialect detection and evidence views.
Their display events can repeat one raw usage record across several content blocks, and
preview limits can truncate or omit large records.
urollup can reuse their format knowledge and tests, but accounting must read source
records rather than sum display events.
Pi’s event stream and its persistent branched sessions need separate fixtures, as do
Codex’s `exec` output and its rollouts.

The agents’ own source is the reference for their dialects.
Codex’s Rust protocol, rollout and hook types can be ported directly, but Codex checks
in no sample rollout files; its tests build records inline.
Pi computes per-file session totals that serve as a reconciliation check for a
`pi-session` adapter, although a fork’s total includes its copied history.

### ccusage Feature Inventory

*(Added 2026-09-15.)* The maintainer asked that urollup roll up usage at least as
effectively as ccusage, so this inventory records ccusage’s user-facing surface at a
pinned release. v20.0.20 (tag `v20.0.20`, commit `bd7f89b`, published to npm on
2026-08-15) was still the latest release on 2026-09-15. The inventory was read from its
CLI parser, command, configuration, output, cost and adapter crates, its npm package and
its guide. Where the guide disagrees with the code, the code is recorded.
The design’s
[ccusage use-case coverage](../../urollup-design.md#106-ccusage-use-case-coverage)
records how urollup handles each use case, and the plan’s
[ccusage reconciliation harness](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#ccusage-reconciliation-harness)
compares results against this release.

**Commands.** The command tree is defined in
[cli-commands.json](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-cli-parser/src/cli-commands.json)
and
[parser.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-cli-parser/src/parser.rs):

| Command path | Report | Notes |
| --- | --- | --- |
| `ccusage`, `ccusage daily`, `weekly`, `monthly`, `session` | Unified report over every detected agent | No command means `daily`; `--sections` prints several reports from one load, and `--by-agent` adds per-agent rows |
| `ccusage claude daily`, `weekly`, `monthly`, `session` | Claude Code only | `daily` adds `--instances`, `--project` and `--project-aliases`; `weekly` adds `--start-of-week`, Sunday by default; `session --id` lists one session’s entries |
| `ccusage codex daily`, `monthly`, `session` | Codex only | No `weekly`; `--speed auto\|standard\|fast`; `--order`, `--mode` and `--breakdown` are ignored |
| `ccusage <agent> daily`, `monthly`, `session` | One of 14 other agents | OpenCode also has `weekly`; Pi takes `--pi-path` and OpenClaw `--open-claw-path` |
| `ccusage blocks`, `ccusage claude blocks` | Claude Code 5-hour blocks inferred from activity | `--active`, `--recent`, `--token-limit` and `--session-length`; `--live` was removed in v18.0.0 |
| `ccusage statusline` | One line for a Claude Code status line command | Own flags, described below |

**Agents.** The 16 built-in agents are
[`BUILT_IN_AGENT_NAMES`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/lib.rs#L57-L60),
each with a directory variable in the
[environment variable guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/environment-variables.md):

- **Agents urollup also reads:** Claude Code (`CLAUDE_CONFIG_DIR`, a comma-separated
  list; defaults `$XDG_CONFIG_HOME/claude`, `~/.config/claude` and `~/.claude`), Codex
  (`CODEX_HOME`; `sessions/`, `archived_sessions/`, or a directory of
  `codex exec --json` output) and Pi (`PI_AGENT_DIR` or `--pi-path`, plus extra
  Pi-format stores declared in the configuration file for unified reports).
- **Agents urollup does not read:** OpenCode, Amp, Droid, Codebuff, Hermes Agent, Goose,
  OpenClaw, Kilo, Kimi, Qwen, GitHub Copilot CLI, Gemini CLI and Grok Build CLI. The
  OpenCode, Hermes, Goose and Kilo adapters read SQLite databases.
  Amp and Codebuff rows add credits, and Hermes rows add message counts.

**Shared report flags.**
[`parse_shared_arg`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-cli-parser/src/parser.rs#L713-L745)
accepts these on every report command:

- **Dates:** `--since` and `--until` take `YYYYMMDD` or `YYYY-MM-DD` and include both
  local dates in `--timezone`, the system zone by default.
  `--last <N>` selects the N most recent periods of the report’s unit.
- **Output:** `--json`; `--jq <filter>`, which pipes the JSON through an external `jq`
  binary
  ([output.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/output.rs#L114-L140));
  `--order asc|desc`, ascending by default; `--breakdown` for per-model rows;
  `--compact`; `--no-cost`, which also strips cost fields from JSON; and `--color` and
  `--no-color`, with `NO_COLOR` and `FORCE_COLOR`.
- **Cost:** `--mode auto|calculate|display`
  ([cost.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/cost.rs#L19-L33)):
  `display` uses a record’s `costUSD` or 0, `auto` uses `costUSD` when present and
  otherwise prices tokens, and `calculate` always prices tokens.
  `--offline` uses the embedded price snapshots instead of fetching LiteLLM prices.
- **Other:** `--config <path>`, `--debug` (scan counts on stderr), `--debug-samples`
  (parsed but unused) and `--single-thread`.
- **Removed:** `--locale` is rejected
  ([tests.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-cli-parser/src/tests.rs#L940)).

**Report fields.**

- **Tables:** period, Models, Input, Output, Cache Create, Cache Read, Total Tokens and
  Cost (USD) columns, plus Last Activity for sessions and a totals row.
  `--breakdown` adds model rows and `--instances` adds project header rows.
  Below 100 terminal columns, or with `--compact`, the layout keeps only period, models,
  input, output and cost, and a pipe never triggers it
  ([should_use_compact_layout](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/output.rs#L18-L25)).
- **Claude JSON:** `{daily|weekly|monthly: [...], totals}`, with rows of `inputTokens`,
  `outputTokens`, `cacheCreationTokens`, `cacheReadTokens`, `totalTokens`, `totalCost`,
  `modelsUsed` and `modelBreakdowns[]`. Session rows add `sessionId`, `firstActivity`,
  `lastActivity` and `projectPath`, and `--instances` nests rows under `projects`.
- **Unified JSON:** rows carry `period` and `agent`, session rows sit under the singular
  key `session`, and `metadata` carries per-source `lastActivity`, `credits` and Codex
  `reasoningOutputTokens`.
- **Codex JSON:** `inputTokens` is uncached input and `cacheCreationTokens` is always 0.
  Rows add `reasoningOutputTokens`, `costUSD` and `models` entries with an `isFallback`
  flag, and session rows add `sessionFile` and `directory`
  ([report.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/report.rs#L55-L80)).
- **Not reported:** there is no cache hit rate, no output schema, and no Markdown or CSV
  output. A missing price prints a stderr warning while the cost shows 0.

**Status line.**
[`run_statusline`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L318)
and the
[statusline guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/statusline.md)
define the command:

- **Input:** it reads only `session_id`, `transcript_path`, `model`, and, when present,
  `cost.total_cost_usd`, `context_window` and `effort.level` from the hook input
  ([`StatuslineHook`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L811-L819)).
- **Output:** model and effort, then session cost, chosen by
  `--cost-source auto|ccusage|cc|both`, where `cc` is Claude Code’s own estimate.
  It adds today’s Claude Code cost in `--timezone`, the active 5-hour block’s cost and
  time left, a burn rate styled by `--visual-burn-rate`, and context tokens with a
  percentage colored by `--context-low-threshold` and `--context-medium-threshold`.
- **Context fallback:** without `context_window`, context is the transcript’s last
  assistant usage against the pricing dataset’s context limit for the model, else
  200,000 tokens
  ([mod.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L630-L660)).
- **Caching and pricing:** output is cached per session in the temporary directory’s
  `ccusage-semaphore/` and refreshed after `--refresh-interval` (1 second) or when the
  transcript changes. Pricing is offline by default, and configured `pricingOverrides`
  are ignored until `main` commit `aa912ef`.
- **Hook data it ignores:** Claude Code’s status line input also carries `rate_limits`,
  the recorded `used_percentage` and `resets_at` of the five-hour, seven-day and
  spend-limit windows for Pro and Max subscribers, and `prompt_cache` statistics such as
  `hit_ratio`. Claude Code debounces runs at 300 ms and cancels a run still in progress
  when a newer update arrives
  ([status line docs](https://code.claude.com/docs/en/statusline), retrieved
  2026-09-15). ccusage reads neither object.

**Configuration.**

- **Files:** `--config`, else the first of `./.ccusage/ccusage.json` and `ccusage.json`
  in each Claude Code configuration directory
  ([config.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-config/src/config.rs#L219-L230)).
  Files are not merged.
- **Option layers:** options apply from `defaults`, through per-command and per-agent
  sections, to CLI flags, which win.
  `pricingOverrides` set rates per raw model name, and the
  [config schema](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/apps/ccusage/config-schema.json)
  is the only JSON schema ccusage publishes.
- **Environment:** `LOG_LEVEL` hides titles and progress, and the undocumented
  `CCUSAGE_MODEL_ALIASES` renames models in reports.
  The guide lists a `CCUSAGE_OFFLINE` variable that no code reads.

**Integrations and distribution.**

- **MCP:** 20.0.20 has no MCP server.
  The `@ccusage/mcp` package shipped through v18.0.11 and was removed in v19.0.0 by
  commit
  [`d7e6993`](https://github.com/ccusage/ccusage/commit/d7e6993cc57852e693b0df86ab3904e3c118fa97).
- **Library API:** there is none.
  The
  [npm package](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/apps/ccusage/package.json)
  ships only a Node launcher and the config schema, with the six native binaries as
  optional dependencies of the same exact version and no install scripts, so
  `npm ci --ignore-scripts` installs a runnable binary.
- **Other channels:** `npx`, `bunx` and `pnpm dlx` runners, which resolve the latest
  version unless pinned, and a Nix flake.

**Comparison notes.** A side-by-side comparison must account for these behaviors, in
addition to the bugs under [Existing Implementations](#existing-implementations):

- `--until` names an inclusive local date, while urollup intervals are half-open.
- Unified reports include every detected agent, so a comparison isolates `HOME` and the
  agent directory variables, or uses the per-agent command paths.
- Without `--offline`, every run fetches LiteLLM prices from the `main` branch and falls
  back silently to embedded data on failure.
- Session reports filter by last-activity date and drop sessions with zero tokens.
  `claude session` always sorts by descending cost, and `claude weekly` starts weeks on
  Sunday, while unified weeks start on Monday.
- Codex reports ignore `--mode` and `--breakdown`, always show 0 cache creation tokens
  at 20.0.20, and mark a model assumed for records without one as a fallback.
- `<synthetic>` is excluded from model lists, and advisor iterations become separate
  model entries.

*(Checked 2026-09-15: `main` at commit
[`a26f517`](https://github.com/ccusage/ccusage/commit/a26f5173fb425beb527b62f118e4275237368efa)
is 234 commits ahead of v20.0.20 and unreleased.
Besides the fixes cited above (`a4b8420`, `15b3bef` and `809eeb6`), it adds a ZCode
adapter
([`562d0ec`](https://github.com/ccusage/ccusage/commit/562d0ecd28fa0788f4510d1640df51869ac24136)),
Antigravity SQLite usage
([`c951e20`](https://github.com/ccusage/ccusage/commit/c951e20dbe60155f1e5df63399f2b1217f797346)),
Copilot session-state events
([`8841f92`](https://github.com/ccusage/ccusage/commit/8841f9211d7225a32afc447de71b76843802b314)),
status line pricing overrides
([`aa912ef`](https://github.com/ccusage/ccusage/commit/aa912efb2f35ab5bcfcc87315c36740b6c2478be)),
Claude Code session totals and Codex history scoped to the date window
([`b2809fa`](https://github.com/ccusage/ccusage/commit/b2809fa580962f39483a3a0e3fca937c74de7dcb),
[`527ec3a`](https://github.com/ccusage/ccusage/commit/527ec3a9cefa28391664b5c0c0ce1aa006264769)),
rejection of invalid date bounds
([`d40d20e`](https://github.com/ccusage/ccusage/commit/d40d20eb88b85157b1cace795d3d660b28ab7d9b))
and unified sessions sorted by cost
([`ff0f032`](https://github.com/ccusage/ccusage/commit/ff0f032ca2dd261139e5d13246bf6290aa4ad7d9)).
The rest are price snapshot refreshes, dependency updates, CI changes, and Codex
originator breakdowns that were added and then reverted.
No 20.0.20 fact in this brief changed.)*

### Log Dialects and Session Linkage

Field names below come from the Codex and Pi source at pinned commits, the ccusage,
agentfdr and session-report parsers and tests, vendor documentation, and key-name
inspection of local logs (see [Methodology](#methodology)); no values, paths or IDs were
copied. Claude Code’s source is not public, and it calls its transcript entry format
internal and subject to change between releases
([sessions](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)), so
Claude facts rest on documentation, third-party parsers and local key names.
Codex documents no stable rollout format either, and Pi session files do not record the
Pi version that wrote them, so each adapter must record the agent versions its fixtures
cover.

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
| `claude-project` | Claude Code sessions with persistence | `<config>/projects/<project>/<session-id>.jsonl`, `<config>` being `CLAUDE_CONFIG_DIR` or `~/.claude` (ccusage also searches `$XDG_CONFIG_HOME/claude`), and `<project>` the cwd with non-alphanumerics replaced by `-`; subagents in `<session-id>/subagents/agent-<agent-id>.jsonl` beside `agent-<agent-id>.meta.json`, and under `subagents/workflows/<workflow-id>/` | One entry per message, content block or metadata record, with `type`, `uuid`, `parentUuid`, `sessionId`, `timestamp`, `cwd`, `gitBranch`, `version` and `isSidechain`; `progress` records can nest a subagent’s assistant record; user entries carry `promptId` |
| `claude-stream` | `claude -p --output-format stream-json --verbose`, saved by the caller | Explicit input only | SDK messages: `system` `init`; `assistant` and `user` with `session_id` and `parent_tool_use_id`; a final `result` with `usage`, `modelUsage`, `total_cost_usd`, `num_turns` and `duration_ms` |
| `codex-rollout` | Codex CLI, IDE extension, Desktop and non-ephemeral `codex exec` runs | `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<local-time>-<thread-id>.jsonl`, with local-time date directories; archiving moves files into flat `archived_sessions/`; a thread revert adds a `_<rollout-id>` file; a default-off feature compresses week-old files to `.jsonl.zst` | `{timestamp, ordinal, type, payload}` lines, `ordinal` only in paginated history (0.145+): `session_meta`, `turn_context`, `response_item`, `event_msg`, `compacted`, `token_usage_record`, `inter_agent_communication`, `world_state`, `retained_context`, `security_risk_score`, `realtime_item` and others |
| `codex-exec` | `codex exec --json`, saved by the caller | Explicit input only | `thread.started` (`thread_id`), `turn.started`, `item.started`, `item.updated`, `item.completed`, `turn.completed` (`usage`), `turn.failed` and `error`; one turn per stream; no timestamps, model, turn or response IDs |
| `pi-session` | Pi sessions with persistence | `$PI_CODING_AGENT_DIR/sessions/--<cwd>--/<timestamp>_<session-id>.jsonl`, default `~/.pi/agent/sessions`; `--session-dir`, `PI_CODING_AGENT_SESSION_DIR` or a `settings.json` `sessionDir` selects one flat directory for every cwd | A `session` header (optional format `version`, `id`, `timestamp`, `cwd`, optional `parentSession`), then tree entries with `id` and `parentId`: `message`, `model_change`, `thinking_level_change`, `compaction`, `branch_summary`, `custom`, `custom_message`, `label`, `session_info` and others |
| `pi-events` | `pi --mode json`, saved by the caller | Explicit input only | The session header, even for `--no-session` runs, then `agent_start`, `turn_start`, `message_start`, `message_update`, `message_end`, `tool_execution_*`, `turn_end`, `agent_end`, `compaction_*` and `auto_retry_*` events |

Pi also emits the same events, interleaved with command responses, in `--mode rpc`, a
third captured shape that is not yet a supported dialect.
An experimental Pi v4 session store behind `PI_EXPERIMENTAL=1` writes explicit usage
rows and, when importing an older file, one aggregate adjustment row that is not a
request
([storage.ts](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/harness/session/jsonl/storage.ts#L264-L300)).

Sources: Claude Code [directory](https://code.claude.com/docs/en/claude-directory),
[subagent](https://code.claude.com/docs/en/sub-agents) and
[SDK message](https://code.claude.com/docs/en/agent-sdk/typescript) docs, and the
ccusage
[Claude notes](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/README.md);
Codex `rust-v0.154.0`
[recorder](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1635-L1657),
[archiving](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/archive_thread.rs#L80-L119),
[rollout payloads](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/rollout_payload.rs#L21-L63)
and
[exec events](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs);
Pi v0.85.1
[session format](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/session-format.md),
[settings](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/settings.md#L250-L256)
and
[JSON mode](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/json.md).

The source shows reader hazards that no format table captures:

- **Time:** Codex names files and date directories in local time but writes UTC line
  timestamps with milliseconds, and a line’s `timestamp` is its write time, so copied
  history carries the copy’s time.
  ccusage accepts only 0 or 3 fractional digits; a reader should accept any RFC 3339
  precision.
- **Malformed and torn lines:** Codex decodes lines through `serde_json::Value`, skips
  malformed lines and takes the first `session_meta` as canonical
  ([recorder.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1026-L1089)).
  When Pi loads a file whose final line is torn, it appends a newline, turning the
  partial write into a permanent malformed interior line
  ([session-manager.ts](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L513-L557)).
  Readers should count such lines per source rather than fail or drop them silently.
- **In-place rewrites:** Pi rewrites a v1 or v2 file with newly minted random entry IDs
  when it loads it
  ([session-manager.ts](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L230-L291)),
  and `codex migrate-rollouts --apply` rewrites legacy rollouts as paginated at the same
  path, dropping usage records of rolled-back turns and trimming a legacy subagent’s
  inherited history
  ([rollout_migration.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration.rs#L1-L9)).
  Both are replacements of a source, not appends.
- **Prefilters and nulls:** a byte-pattern prefilter may only route lines to full
  parsing, and a null nested field is never a reason to reject a record, as ccusage’s
  dropped records show.
- **Unwritten sessions:** Codex creates a rollout only when it first persists a thread,
  and Pi creates a session file only at the first assistant message; Codex `--ephemeral`
  threads and Pi `--no-session` runs write no file.

#### Usage Fields

| Usage | `claude-project` | `codex-rollout` | `pi-session` |
| --- | --- | --- | --- |
| Request identity | `requestId` and `message.id` on each assistant entry | `token_usage_record` payload (since `rust-v0.153.0`): `response_id`, `thread_id`, `turn_id`, `root_turn_id`, `session_id`; older rollouts have only `token_count` events, with no response ID | `message.responseId`, absent for Bedrock and older releases; entry `id` is unique only within one file |
| Fields | `message.usage`: `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `output_tokens`, `cache_creation.ephemeral_5m_input_tokens` and `ephemeral_1h_input_tokens`, `output_tokens_details.thinking_tokens`, `server_tool_use`, `service_tier`, `speed`, `inference_geo`, `iterations` | `input_tokens`, `cached_input_tokens`, `cache_write_input_tokens` (0.145+), `output_tokens`, `reasoning_output_tokens`, `total_tokens`; records add `turn_token_usage` and `thread_token_usage` running sums | `usage`: `input`, `output`, `cacheRead`, `cacheWrite`, optional `cacheWrite1h` and `reasoning`, `totalTokens`, and a `cost` breakdown computed by Pi |
| Carriers | Assistant entries; `advisor_message` items in `iterations` carry a second model’s usage | `token_usage_record` payloads and `token_count` events; `compacted.latest_token_usage_record` is a copy | Assistant messages; since 0.81.0 also `toolResult` messages, `compaction` entries and `branch_summary` entries, all without a model |
| Inclusion | `input_tokens` excludes cache reads and writes; top-level counts equal the sum of `message` iterations and exclude `advisor_message` iterations | `input_tokens` includes `cached_input_tokens`, and whether it includes `cache_write_input_tokens` is unverified; `reasoning_output_tokens` is part of `output_tokens` | `input` excludes `cacheRead` and `cacheWrite`; `reasoning` is part of `output`; `totalTokens` is provider-reported for Google, Bedrock and Mistral and can differ from the component sum |
| Counter kind | Per response, repeated on every content-block entry of one request | `token_count.info.total_token_usage` is cumulative per thread and `last_token_usage` the latest delta, with `info: null` before the first recorded usage; `token_usage_record.usage` is per response | Per message, written once in the file that recorded it |
| Model and effort | `message.model`; entry-level `effort` | `turn_context` `model`, the requested model because the served model is not saved, and `effort`; `thread_settings_applied` service-tier changes | `message.provider`, `message.model`, optional `responseModel` and `providerThinkingLevel`; `model_change` and `thinking_level_change` entries |
| Provider windows | `quotaLimits` on some assistant entries: `rateLimitType`, `resetsAt`, `status`; usage-limit error text with a reset time | `token_count.rate_limits`: `limit_id`, `plan_type`, `credits`, and `primary` and `secondary` windows with `used_percent`, `window_minutes`, `resets_at` | None recorded |

Evidence and caveats for the usage fields:

- **Claude:** Anthropic defines `input_tokens` as tokens after the last cache breakpoint
  ([prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)).
  In a small local sample, all entries sharing a `requestId` had one `message.id` and
  identical usage, but Anthropic’s receipts miner reports that about 13% of block
  records for one response disagree on `output_tokens` while it streams
  ([mine-transcripts.mjs](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L412-L432)).
  Parsers resolve that disagreement differently: ccusage keeps the one whole record with
  the largest token total, session-report the largest `output_tokens` within a file, and
  receipts a field-wise maximum that can combine values no single record held.
  A reconciler should select one record per request by a documented rule and diagnose
  the disagreement. ccusage’s
  [advisor fixture](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L320-L343)
  shows that top-level usage excludes `advisor_message` iterations, which carry their
  own `model`; agentfdr instead uses the last iteration’s input and cache counts for the
  whole request. ccusage prefers the `cache_creation` duration breakdown over
  `cache_creation_input_tokens` when both exist, so a mismatch needs a diagnostic, and
  receipts found 1-hour and 5-minute cache writes near an even split on one corpus, so
  pricing all writes at the 5-minute rate misprices them.
  `effort`, `iterations`, `speed`, `inference_geo` and `quotaLimits` were observed
  locally and are undocumented, and `message.model` can be `<synthetic>`.
- **Codex:** The protocol defines `TokenUsage`, `TokenUsageRecord`, `SessionMeta` and
  the rate-limit types
  ([protocol.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2215-L2391)),
  and values are copied from the Responses API `usage`. `token_usage_record` first
  shipped in `rust-v0.153.0`. Codex writes one per completed response that reports
  usage; responses without usage, including legacy remote compaction, write none
  ([session/mod.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4377-L4409)).
  `token_count` events also appear without a new request: with `info: null` as a
  rate-limit observation before a thread’s first recorded usage, with unchanged `info`
  on rate-limit updates, with a compaction estimate as a `last_token_usage` holding only
  `total_tokens`, and on a full context window with the cumulative total set to the
  window size and its components zeroed
  ([session/mod.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4447-L4536),
  [turn.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L1484-L1494)).
  The last case explains the decreasing cumulative total seen locally, so a delta is
  valid only within a **counter epoch**, a span in which no cumulative component
  decreases. Otherwise local `last_token_usage` records agreed with agentfdr’s
  [codex.js](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js)
  notes: cached input never exceeded input, reasoning never exceeded output, and about
  99% had `total_tokens` equal to input plus output.
  Each `token_count` repeats the session’s one latest rate-limit snapshot, whose
  carried-forward `plan_type` and `credits` may be stale, and only the last `limit_id`
  bucket of a response reaches the rollout
  ([state/session.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L388-L411)).
  Rollouts before 2025-09-06 contain no `token_count` events
  ([Codex guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/codex/index.md)).
- **`codex-exec`:** `turn.completed.usage` is the thread’s cumulative total, not the
  turn’s. It has no `total_tokens`, excludes subagent threads, includes earlier runs
  after `codex exec resume` because the total is seeded from the rollout, and is absent
  for failed turns; interrupted turns emit no terminal event
  ([event processor](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L117-L128),
  [session/mod.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1464-L1471)).
  A turn’s own usage is the difference between consecutive totals for one `thread_id`. A
  non-ephemeral `codex exec` run also writes a rollout, and the only key the two share
  is `thread.started.thread_id`, equal to `session_meta.payload.id`; the rollout owns
  the usage, and the capture is a reconciliation check.
- **Pi:** The v0.85.1
  [message types](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/types.ts)
  add `responseId`, `responseModel`, `providerThinkingLevel`, `cacheWrite1h` and
  `reasoning`, which the 0.62.0 session docs lack.
  Every provider adapter maps `input` without cache reads and writes and counts
  `reasoning` inside `output`, but `totalTokens` equals the component sum by
  construction only for the Anthropic and OpenAI adapters, while Google, Bedrock and
  Mistral report their own totals
  ([Google adapter](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/google-generative-ai.ts#L224-L240)).
  It equaled the sum in the small local sample.
  Before 0.70.0 the completions adapter double-counted reasoning tokens.
  `cost` is an estimate from the installed model catalog at response time, zero can mean
  unpriced, and subscription use is not recorded.
  A session file writes each message’s usage once.
  Repeated usage comes from copies, in fork, clone, `--fork` and export files,
  `compaction.retainedTail` messages and extension `details` payloads, and from
  `pi-events`, which repeats one message’s usage in `message_start`, `message_update`,
  `message_end`, `turn_end` and `agent_end`; only `message_end` is final
  ([agent loop](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/agent-loop.ts#L200-L272)).
  `message_update` dropped its cumulative `message` in 0.84.0 and gained a top-level
  cumulative `usage` in 0.84.2.

#### Session Linkage

| Linkage | `claude-project` | `codex-rollout` | `pi-session` |
| --- | --- | --- | --- |
| Session identity | File name stem; entries copied by resume or fork keep their original `sessionId`, so one file can hold several | `session_meta.payload.id` is the thread and `session_id` the root thread (since `rust-v0.142.0`); revert chains and paginated forks spread one thread over several files, and resume appends to the original file | Header `id`, a UUIDv7 since 0.67.1; `--session-id` accepts caller-chosen IDs that are unique only within a project directory, and export then import writes a second file with the same `id` |
| Subagents | Entries carry `agentId` and `isSidechain: true`; `.meta.json` has `agentType`, `description`, `toolUseId` and `spawnDepth`; `toolUseId` matches the spawning `Agent` `tool_use.id` in the parent or another subagent, whose `toolUseResult` records `agentId`; labeled `agent-a<label>-<hex>` files are internal background forks such as `compact` | `parent_thread_id`; `source.subagent.thread_spawn` with `parent_thread_id`, `depth`, `agent_nickname`, `agent_role` and `agent_path`; other sources `review`, `compact`, `memory_consolidation` and `other`; `thread_source` is `user`, `subagent`, `guardian_review`, `memory_consolidation` or a feature name; `token_usage_record.root_turn_id` links a subagent request to its root turn | None built in; Pi’s subagent example runs children with `--no-session`, so their usage survives only inside the parent’s tool-result `details` |
| Forks and resumes | `/branch` and `--fork-session` create a new session ID holding copied history; `--resume` keeps the ID; fork-style subagents replay parent entries with identical `uuid`s; older transcripts keep subagent turns inline, marked `isSidechain`, with no spawn ID | `forked_from_id` with `forked_from_ordinal_exclusive` (0.152+); `subagent_history_start_ordinal` (0.145+, paginated history) marks where a subagent’s own records begin | Header `parentSession`, the source file’s absolute path, for `/fork`, `/clone` and `--fork`; in-file branches through `parentId` and `branch_summary.fromId` |
| Directory and branch | Entry `cwd` and `gitBranch` | `session_meta` `cwd` and `git` (`commit_hash`, `branch`, `repository_url`) per thread; `turn_context.cwd` per turn | Header `cwd`; no branch |

Copies that must not add usage, and the evidence that identifies them:

- **Claude Code:** Content-block records of one response share `message.id` and
  `requestId`. A parent transcript can hold `progress` records whose `data.message`
  nests a subagent’s assistant record, duplicating the subagent file
  ([ccusage fixture](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L425-L484)).
  `/btw` side-question logs, `isSidechain` files under `subagents/`, replay parent
  messages with the same `message.id` and a different `requestId`
  ([ccusage notes](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/README.md#L19-L31)).
  Fork-style subagents replay parent entries with identical `uuid`s
  ([analyzer](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L541-L554)),
  so `uuid` equality is lineage evidence even when `requestId` differs.
  Some gateways reuse one `message.id` across sessions without a `requestId`, so that
  key alone is ambiguous rather than proof of a copy.
- **Codex:** Key requests by `token_usage_record.response_id` and attribute each to the
  record’s `thread_id`; a record whose `thread_id` differs from the file’s thread is a
  copy. Legacy user forks copy the parent rollout, records and `token_count` events
  included, while paginated forks copy nothing and reference the parent file through
  `history_base`
  ([thread_manager.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/thread_manager.rs#L1332-L1397)).
  Subagent forks always drop `token_usage_record` but copy the parent’s `session_meta`
  as a foreign line and, in legacy history, the parent’s `token_count` events, so the
  child’s running total continues the parent’s
  ([spawn.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105)).
  Copied lines keep the parent’s turn IDs but get new write-time timestamps.
  A child’s own records begin at `subagent_history_start_ordinal`, else at the first
  `thread_settings_applied` whose `thread_id` names the child (0.152+), else at a
  heuristic boundary labeled inferred.
  Before `rust-v0.142.0`, `session_meta` has no `session_id`, and Codex’s fallback to
  `id` is wrong for subagents, so roots come from parent links.
  Parallel guardian reviews and `--ephemeral` threads never reach disk.
- **Pi:** `/fork` and `/clone` copy the root-to-current path into a new file, keeping
  each entry’s `id`, message object and timestamp, and `--fork` copies every branch
  verbatim
  ([session-manager.ts](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L1422-L1662)).
  Export keeps the header `id` and entry IDs with no `parentSession`. Entry IDs are 8
  hex characters checked for collisions only within one file, so they collide across a
  large corpus. A workable rule keys requests by provider and `responseId`, else by
  lineage root plus a digest of copy-invariant fields; resolves `parentSession` by file
  basename, because the stored path is machine-specific; assigns copied history to the
  parent; and never counts nested copies.

Claude Code documents the subagent directory and resume behavior
([subagents](https://code.claude.com/docs/en/sub-agents),
[sessions](https://code.claude.com/docs/en/sessions)). The `.meta.json` keys, copied
`sessionId` values and inline subagent turns come from local inspection and agentfdr’s
[subagents.js](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js).
In one inspected local session, each `.meta.json` `toolUseId` matched an `Agent` call in
the parent, and subagent entries’ `sessionId` equaled the parent session’s ID. agentfdr
and ccusage also read `subagents/workflows/<workflow-id>/agent-<agent-id>.jsonl`, and
session-report a `<session>/workflows/` variant; the inspected logs contained neither.
In recent local Codex rollouts, every `thread_spawn` source named the same parent as
`parent_thread_id`, and `session_id` equaled `id` only for user threads.

#### Current-Session Signals

| Signal | Claude Code | Codex | Pi |
| --- | --- | --- | --- |
| Tool-subprocess environment | `CLAUDE_CODE_SESSION_ID` in Bash, PowerShell and hook subprocesses, updated on `/clear`; `CLAUDECODE` and `CLAUDE_CODE_CHILD_SESSION` mark nesting; `CLAUDE_PID` (v2.1.214+) is the agent process | `CODEX_THREAD_ID`, the current thread, so a subagent’s own thread inside its tools (present by `rust-v0.100.0`); `CODEX_SESSION_ID`, the root session (added after `rust-v0.135.0`); `CODEX_VERSION`; none set for hooks or MCP servers | `PI_SESSION_ID` and `PI_SESSION_FILE` (0.82.0+), with `PI_PROVIDER`, `PI_MODEL` and `PI_REASONING_LEVEL`, only in model-invoked `bash` and `powershell` tools; `PI_SESSION_ID` without `PI_SESSION_FILE` means an unsaved session; `AI_AGENT=pi` marks the process |
| Hook input | `session_id`, `transcript_path`, `cwd`; `SubagentStart` and `SubagentStop` add `agent_id` and `agent_type`, and `SubagentStop` adds `agent_transcript_path` | `session_id`, always the root session; `turn_id`; `transcript_path`, the current thread’s rollout or null when ephemeral; `cwd`; `model`; `agent_id`, the child thread, and `agent_type` for spawned subagents; `SubagentStop` sets `transcript_path` to the parent’s rollout and `agent_transcript_path` to the child’s | Extension API only |
| ID to transcript | `<config>/projects/*/<id>.jsonl`; `--resume <id>` searches the current project and its worktrees first | File name ending `-<thread-id>.jsonl`, or `_<rollout-id>.jsonl` after a revert, possibly `.zst`, under local-time date directories in `sessions/` or flat in `archived_sessions/`; SQLite `threads.rollout_path` is an optional hint | `PI_SESSION_FILE` is the path |
| Cloud | `CLAUDE_CODE_REMOTE` and `CLAUDE_CODE_REMOTE_SESSION_ID`; whether that ID names a transcript file is unverified | Not surveyed; the thread store is local or in-memory only, and hook `transcript_path` is null without a local thread | Not applicable |

Sources: Claude Code [environment variables](https://code.claude.com/docs/en/env-vars)
and [hooks](https://code.claude.com/docs/en/hooks); Codex
[shell_environment.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/shell_environment.rs),
[exec_env.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs),
[process_manager.rs](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/unified_exec/process_manager.rs#L1370-L1377),
[hooks schema](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs),
[hook registry](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/registry.rs#L73-L82)
and
[hook runtime](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L385-L444),
with the version bounds checked at the `rust-v0.100.0` and `rust-v0.135.0` tags; Pi
[environment variables](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/environment-variables.md),
[bash tool](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/tools/bash.ts#L166-L192)
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
- The Codex and Pi signals were verified from source and documentation only.
  In Codex source, a subagent’s tools receive the subagent’s own `CODEX_THREAD_ID`, and
  hooks receive no `CODEX_*` variables because they replay the Codex process environment
  captured at session start; a hook of a Codex started from another Codex’s tool shell
  therefore sees the outer Codex’s variables.
  Because hook `session_id` is the root session, comparing it with the transcript’s
  `session_meta.payload.id` fails inside subagents; match it with
  `session_meta.payload.session_id`, and `agent_id` with `session_meta.payload.id`.
- Pi writes the assistant message that called a tool before the tool runs, so a Pi
  current-session summary includes the calling request
  ([agent-session.ts](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L672-L691)),
  whereas a Claude transcript can lag the running turn.
  Pi’s bash tool removes inherited `PI_SESSION_*` and model variables before setting its
  own, but not other agents’ variables, and user-typed `!` commands receive none.
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
count as another. A usage carrier need not be one request either: a Claude advisor
iteration is a second model’s usage inside one request, a Pi compaction entry can
combine two summary calls, and Pi tool-result usage covers an unknown number of calls.

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
their own. Pi’s recorded `cost` is itself an estimate from its installed catalog, and
ccusage writes unpriced tokens as cost 0 in JSON with no flag.
Unknown cost is not zero, and tokens, dollars, wall time and CPU time are separate
units.

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
[session-report analyzer](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L14-L28)
documents both behaviors and cites 3–10x overcounting without request-level
deduplication. It keeps the largest `output_tokens` per request key within a file and
removes replayed records with a global first-seen `uuid` set, so processing order
decides which file owns a replay.
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

The reconciled request selects the final record’s usage and keeps all four records as
evidence references; if block records disagreed on other fields, it would still select
one whole record and report the disagreement.
ccusage 20.0.20 deduplicates Claude entries by message ID and request ID across every
scanned file, falls back to the message ID alone when a log marked `isSidechain` replays
parent messages under new request IDs, and keeps a non-sidechain duplicate over a
sidechain one, then the duplicate with the largest token total
([Claude adapter](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L118-L233)).
Entries without a `message.id` are never deduplicated, and the winning record keeps its
own timestamp, so a request can move across a day boundary.
Such keys are policy choices, not universal identities: ccusage commit
[`a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84)
(#1661) adds the session ID to every key, with a test for gateways that reuse message
IDs across distinct sessions, while copied transcripts that keep the original
`sessionId` still collapse.

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
[Codex parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L320-L367)
uses `last_token_usage` only when the running total advances and otherwise subtracts the
previous total, so event 3 contributes nothing.
It trusts `last_token_usage` even when it disagrees with that difference, so a missing
line loses its gap, and with only a running total, a reset yields zeros rather than a
new counter epoch. A reader must also skip compaction estimates and context-window-full
fills, whose `last_token_usage` carries only a synthetic `total_tokens`. Since
`rust-v0.153.0`, `token_usage_record` gives each response an ID, which reduces this
arithmetic to a fallback for older rollouts.

For copied parent history, the ccusage
[replay plan](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs#L31-L111)
matches a forked or spawned child’s leading usage values against the parent’s events up
to the child’s `session_meta` timestamp, and otherwise skips a leading burst of events
less than 1 s apart
([parser.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L84-L149)).
Both are heuristics: a child whose own first two events fall within a second loses them,
and a compressed replay of a single event is kept.

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
An **observation bundle**, a versioned `*.urollup/` folder of zstd-compressed tables,
carries observation tables and always contains the usage summary computed from them.
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
for urollup. Rust costs slower iteration, and format knowledge from Python, JavaScript
and TypeScript tools must be ported rather than imported, but Codex’s protocol types and
ccusage’s readers are already Rust (see
[Reusable Code and Tests](#reusable-code-and-tests)).

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
In ccusage, the Claude reader holds every usage entry until a report ends while the
Codex reader streams lines into per-worker maps, and with no log cache its statusline
rescans all Claude files whenever its per-session output cache is stale.

Persistent caching should follow a correct uncached reference, but stable source and
observation identities are needed from the start.
A later cache must handle appends, partial lines, in-place replacement such as Codex
rollout migration and Pi’s v1 rewrite, renames such as Codex archiving and compression,
deletion, late corrections, and changes to parser or pricing versions.
Re-importing identical data must never add usage, and cached and uncached reports must
agree for the same source snapshot and query.

### Local Log Volume and Throughput

A spike measured one developer machine’s real logs to decide when the capture cache
ships. The prototype is kept as reference in
[explorations/log-throughput](../../../explorations/log-throughput/README.md); it opens
logs read-only and records aggregates only, so no content, paths or IDs appear here.

**Setup.** Apple M1 Pro, 10 cores, 32 GiB RAM, local SSD, release build, 2026-09-14.
Other agents ran concurrently, so the 1-minute load average stayed around 30–40 during
runs; times are medians of three runs and are conservative.
Every parsing variant produced identical totals on both slices.

**Volume.**

| Measure | Claude Code | Codex |
| --- | --- | --- |
| Files | 1,224 (205 sessions, 1,019 subagent files) | 8,896 rollouts (28 gzip, 69 legacy JSON) |
| Logical bytes | 2.02 GB | 17.5 GB |
| Lines | 0.80 M | 3.73 M |
| Active days, bytes per active day | 38, 53 MB | 132, 133 MB |
| Past 30 days | 1,195 files, 1.99 GB | 2,215 rollouts, 11.2 GB |
| File size p50, p99, max | 0.70 MB, 24 MB, 49 MB | 0.38 MB, 36 MB, 388 MB |
| Largest line | 3.8 MB | 16 MB |

Claude Code’s 30-day transcript cleanup shows directly: nearly all retained Claude bytes
are from the past month, while Codex keeps months of rollouts.

**Composition and captured size.** Usage-bearing records are a small share of bytes:
Codex usage records are 0.59 GB of 17.5 GB, and display and structure records (with
their embedded content) make up most of the rest.
Stripping content to `{bytes, digest}` stubs removed 1.35 GB of Claude text and 15.2 GB
of Codex text.

| Captured records | Claude Code | Codex |
| --- | --- | --- |
| Records kept | 0.64 M | 2.91 M |
| Stripped JSONL | 699 MB | 1.85 GB |
| zstd level 3 | 76 MB | 260 MB |
| zstd level 19 | 68 MB | 216 MB |
| Usage records only, zstd 3 | 46 MB | 44 MB |

The full captured-records cache took 340 MB on disk for 19.5 GB of logs, about 1.7%.
Building it once with zstd level 19 took 792 s of wall time and 1.06 GiB peak RSS on 10
threads; level 3 is the practical default for a cache.

**Throughput.** Runs read 9,207 source files (about 11.4 GiB); peak RSS stayed between
110 and 190 MiB.

| Workload | All history, 10 threads | Past 30 days, 10 threads | All history, 1 thread |
| --- | --- | --- | --- |
| Read and decompress only | 4.5 s | 3.3 s | 17.4 s |
| Parse every line into `serde_json::Value` | 15.6 s | 11.5 s | 63.8 s |
| Borrowed typed parse | 8.3 s | 4.8 s | 36.7 s |
| Byte prefilter, then typed parse | 6.7 s | 3.0 s | 27.9 s |
| Capture cache | 3.5 s | 2.2 s | 11.7 s |
| Capture cache with prefix check | 4.7 s | 2.3 s | 18.1 s |

A first-run probe of the same files took 5.2 s just to read them (3.0 s and 2.2 s on
repeats), and a later probe measured 6.6 s for the typed parse, 4.0 s with the prefilter
and 2.1 s from the capture cache.
The operating system’s page cache could not be dropped without elevated privileges, so
no run is fully cold.

**Reconciliation diagnostics.** The same runs counted, in aggregate, what naive summing
would have miscounted over all history, using the spike’s own keys (which
[differ](../../../explorations/log-throughput/README.md#known-divergences-from-the-urollup-contracts)
from the contract’s rules):

| Diagnostic | Count |
| --- | --- |
| Claude usage observations | 311,532 |
| Claude unique requests | 141,343 |
| Claude duplicates within one file (repeated block records) | 151,193 |
| Claude duplicates across files (replays and copies) | 18,996 |
| Claude duplicates whose usage disagrees | 23,571 |
| Codex `token_count` events | 285,286 |
| Codex identical consecutive snapshots | 16,648 |
| Codex `info: null` events (limit-only) | 3,708 |
| Codex counter resets (new epochs) | 105 |
| Codex forked and subagent rollouts | 538 and 2,215 |
| Codex `token_usage_record` lines, all unique | 81,609 |

Summing every Claude observation would count 2.2 times as many requests as actually
occurred, and about one duplicate in seven disagrees on usage, so the record-selection
rule changes totals.
For Codex, summing cumulative totals per counter epoch and summing per-response
`last_token_usage` differ by about 7 times on the same files, which is why the
contract’s copy, epoch and estimate rules are needed rather than either naive sum.

**Findings.**

- Uncached extraction is already fast: a typed parse with a byte prefilter covers the
  past month in about 3 s and all history in about 7 s on 10 cores under heavy load.
  Borrowed typed decoding plus a prefilter is about 2.3 times faster than generic
  `serde_json::Value` parsing.
- The capture cache saves about 1.4 times on the past month and 1.9 times on all history
  with 10 threads, and about 2.4 times on one thread.
  Its advantage grows on cold page caches, slower disks and constrained CPUs, because it
  avoids reading the 85% of bytes that are content.
- Its larger value is durability: 340 MB preserves re-extractable usage data for logs
  that Claude Code deletes after 30 days.
- Very large Codex rollouts (up to 388 MB, lines up to 16 MB) make streaming line reads
  and per-file parallelism necessary; a whole-file JSON approach would not fit.

**Recommendation.** Performance alone does not require the capture cache in Phase 1: the
uncached engine meets the plan’s targets with prefiltered typed parsing and per-file
parallelism. Captured records should still be written by Phase 1 bundles, and the
default-on cache can follow once the uncached engine is the correctness reference,
unless retaining captured records beyond log deletion is wanted sooner.
*(Decided 2026-09-15: retention was wanted sooner, so the durable capture store ships
default-on in Phase 1, milestone 0.3, writing idle default-discovered sources, and its
cache read path follows in Phase 2; see design
[Decision 8](../../urollup-design.md#decision-8-capture-store-and-cache).)*

### Reusable Code and Tests

The reviewed projects offer code and tests, not only format facts.
The table lists the most valuable items and the urollup bead each affects.
**Port code** copies or closely translates source, which carries license obligations.
**Port logic** reimplements a rule from its description and needs a citation.
**Adapt tests or fixtures** rewrites cases as synthetic urollup fixtures with urollup’s
expected results, including negative cases where the source is wrong.
A **format fact** records dialect behavior, and **learn only** takes a lesson without
code.

Ported code keeps its license notice and attribution, as the
[urollup design](../../urollup-design.md#decision-3-code-reuse-and-licensing) requires,
recorded with the source path and commit in a third-party notices file.
MIT sources need their copyright notice and license text: ccusage (Copyright (c) 2025
ryoppippi), agentfdr (Copyright (c) 2026 kamihork) and Pi (Copyright (c) 2025 Mario
Zechner).
Apache-2.0 sources need the license text and a statement of changes; Codex code
also carries the attribution in its `NOTICE` file (OpenAI Codex, Copyright 2025 OpenAI),
and the Anthropic plugins ship no `NOTICE` file, so their ported code keeps the license
text and attribution.
Pi’s checked-in session fixtures and ccusage’s statusline test inputs hold real prompts
or paths, so fixtures derived from them must keep only structure and usage.

| Source | Item | File (pinned) | Reuse mode | Bead |
| --- | --- | --- | --- | --- |
| Codex | Usage, `token_count`, `token_usage_record` and rate-limit types, with unknown fields kept in a raw map | [protocol.rs:2215-2391](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2215-L2391) | Port code with Apache-2.0 notice | uro-y2qj |
| Codex | `SessionMeta` with its missing-`session_id` fill, and session, subagent and thread source types | [protocol.rs:2740-2840](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2740-L2840), [3034-3171](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3034-L3171) | Port code with Apache-2.0 notice | uro-y2qj, uro-20ck |
| Codex | Exec event and hook input types | [exec_events.rs:8-133](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs#L8-L133), [schema.rs:278-638](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs#L278-L638) | Port code with Apache-2.0 notice | uro-y2qj, uro-20ck |
| Codex | Rollout file-name parser for plain and revert names, relabeling its time as local | [rollout_file_name.rs:39-60](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/rollout_file_name.rs#L39-L60) | Port code with Apache-2.0 notice | uro-20ck |
| Codex | Value-first line decoding and legacy normalizers for RFC 3339 `resets_at` and retired records | [lib.rs:39-73](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/lib.rs#L39-L73), [line_parser.rs:33-135](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/line_parser.rs#L33-L135) | Port logic; adapt tests or fixtures | uro-y2qj |
| Codex | Plain-over-compressed discovery and multi-frame zstd reading | [compression.rs:144-192](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L144-L192), [seekable_reader_tests.rs:11-79](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/seekable_reader_tests.rs#L11-L79) | Port logic; adapt tests or fixtures | uro-20ck, uro-gnqj |
| Codex | Token scenarios: records across resume, `info: null`, context-full fills, compaction estimates, subagent non-inheritance and per-type wire shapes | [token_usage_rollout.rs:31-116](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/token_usage_rollout.rs#L31-L116), [session/tests.rs:2808-2936](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/tests.rs#L2808-L2936), [control_tests.rs:1388-1523](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control_tests.rs#L1388-L1523), [client.rs:3441-3532](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/client.rs#L3441-L3532), [history tests.rs:393-518](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/tests.rs#L393-L518) | Adapt tests or fixtures | uro-obx5, uro-spce |
| ccusage | Ordered, size-balanced parallel file reader, with panics replaced by errors and worker limits added | [lib.rs:49-126](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/common/src/lib.rs#L49-L126) | Port code with MIT attribution | uro-y2qj |
| ccusage | `memmem` line prefilter and line splitter, used only to route lines to full parsing | [fast.rs:17-104](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/fast.rs#L17-L104) | Port code with MIT attribution | uro-y2qj, uro-gnqj |
| ccusage | ANSI-aware width, truncation and boxed tables, writing to an injected writer and stripping control characters from log strings | [width.rs:1-161](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-terminal/src/width.rs#L1-L161), [table.rs](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-terminal/src/table.rs) | Port code with MIT attribution | uro-d135 |
| ccusage | `fs_fixture!` test macro, without the process-environment guard | [lib.rs:81-142](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-test-support/src/lib.rs#L81-L142) | Port code with MIT attribution | uro-phi8, uro-obx5 |
| ccusage | Claude duplicate resolution (sidechain precedence, whole-record winner), rebuilt as order-independent grouping | [lib.rs:118-233](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L118-L233) | Port logic | uro-spce |
| ccusage | Advisor iterations and `progress`-embedded subagent messages | [lib.rs:306-404](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L306-L404), [daily.rs:140-190](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L140-L190) | Port logic; format fact | uro-y2qj |
| ccusage | Codex cumulative-to-delta rule, extended with counter epochs and synthetic-event diagnostics | [parser.rs:320-367](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L320-L367) | Port logic | uro-y2qj, uro-spce |
| ccusage | Codex fork replay value matching and burst heuristic, as a last-resort fallback labeled inferred | [replay.rs:31-111](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs#L31-L111), [parser.rs:84-221](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L84-L221) | Port logic | uro-spce |
| ccusage | Dedupe cases: complete and requestless duplicates, advisor and `progress` records, sidechain replay, Codex fork and legacy subagent replay, repeated snapshots, tier transitions | [main.rs:257-484](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L257-L484), [lib.rs:690-782](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L690-L782), [loader.rs:198-2011](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L198-L2011) | Adapt tests or fixtures, adding ccusage’s bugs as negative cases | uro-obx5 |
| ccusage | Claude and Codex root discovery, including `$XDG_CONFIG_HOME/claude`, a `projects/` directory value and archived rollouts | [claude paths.rs:12-68](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/paths.rs#L12-L68), [codex paths.rs:20-116](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/paths.rs#L20-L116) | Port logic | uro-20ck |
| ccusage | DST-safe local-midnight date bounds, rejecting unknown zones instead of falling back | [date_utils.rs:212-241](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L212-L241) | Port logic | uro-d135 |
| Pi | Session, entry and message types, and per-provider usage mapping | [session-manager.ts:32-153](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L32-L153), [types.ts:383-468](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/types.ts#L383-L468), [openai-responses-shared.ts:556-582](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/openai-responses-shared.ts#L556-L582) | Format fact | uro-qnok |
| Pi | Fork, clone, `--fork` and export copy rules | [session-manager.ts:1422-1662](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L1422-L1662), [tree-traversal.test.ts:462-516](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/tree-traversal.test.ts#L462-L516) | Format fact; adapt tests or fixtures | uro-qnok, uro-spce |
| Pi | Per-file session totals as a reconciliation check | [usage-totals.ts:22-70](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/usage-totals.ts#L22-L70), [agent-session.ts:3326-3381](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L3326-L3381) | Port logic | uro-qnok |
| Pi | Cache-miss estimator, labeled an estimate | [cache-stats.ts:1-164](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/cache-stats.ts#L1-L164), [cache-stats.test.ts:60-143](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/cache-stats.test.ts#L60-L143) | Port code with MIT attribution | uro-jpmf, uro-d135 |
| Pi | Torn-tail repair and v1-to-v3 migration cases, generated as synthetic files | [file-operations.test.ts:71-103](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/file-operations.test.ts#L71-L103), [migration.test.ts:5-78](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/migration.test.ts#L5-L78) | Adapt tests or fixtures | uro-obx5, uro-qnok |
| agentfdr | Anomaly detectors (tool loops with a retry allowance, error streaks, token spikes, stalled calls, refusals) and strict detector configuration | [detect.js:11-581](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/detect.js#L11-L581), [config.js:1-109](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/config.js#L1-L109), [detect.test.js:71-252](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/detect.test.js#L71-L252) | Port logic; adapt tests or fixtures | uro-jpmf |
| agentfdr | Subagent discovery, spawn placement and inline sidechain nodes | [subagents.js:3-229](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js#L3-L229), [subagents.test.js:21-127](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/subagents.test.js#L21-L127) | Port logic; adapt tests or fixtures | uro-20ck |
| session-report | Subagent type resolution: `.meta.json` `agentType`, file-name label, the spawning call’s `subagent_type`, else `fork` | [analyze-sessions.mjs:111-156](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L111-L156), [247-265](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L247-L265) | Port logic | uro-20ck, uro-52qi |
| session-report | Prompt and skill attribution windows, labeled estimates | [analyze-sessions.mjs:420-480](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L420-L480) | Port logic | uro-52qi, uro-jpmf |
| session-report and receipts | Skill model: CLI JSON, fixed template with short narrative slots, and honesty rules | [SKILL.md:1-42](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/SKILL.md#L1-L42), [template.html:315-352](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/template.html#L315-L352), [receipts SKILL.md:139-164](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L139-L164) | Learn only | uro-jpmf |
| receipts | Block-record `output_tokens` disagreement and the 1-hour and 5-minute cache-write split | [mine-transcripts.mjs:170-212](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L170-L212), [412-432](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L412-L432) | Format fact | uro-y2qj, uro-wuby |

## Options Considered

The approaches differ in architecture; any new code is Rust (see
[Rust as a Constraint](#rust-as-a-constraint)).

### Option A: Wrap Existing CLIs

**Description:** Run ccusage, agentfdr and similar tools and combine their reports.

**Pros:**

- Useful reports immediately

**Cons:**

- The tools’ identities, deduplication keys, scopes and pricing semantics differ and
  stay different, and ccusage’s own reports disagree across command paths

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

- Build a narrow shared engine: port the items in
  [Reusable Code and Tests](#reusable-code-and-tests) with attribution, use existing
  tools as behavioral references rather than accounting authorities, and use
  Metabrowser’s adapter boundary as a design reference.
- Prefer native identities and boundaries over heuristics: Codex `response_id` and
  `subagent_history_start_ordinal` before value matching or timing, Pi `responseId`
  before digests, with every heuristic exclusion labeled inferred.
- Support Claude Code and Codex first, then add Pi and imported cloud formats as each
  passes its own fixtures.
  Scope support by validated dialect and metric, not by vendor.
- Encode the [synthetic double-counting cases](#synthetic-double-counting-example) and
  the copy rules under [Session Linkage](#session-linkage) as golden fixtures before
  building reports on any adapter.
- Ship uncached CLI reports, usage summaries and observation bundles first, then the
  read-only web UI and workflow skill, then persistent caching.
  *(Decided 2026-09-15: the capture store also ships in Phase 1; its cache read path
  arrives with the web UI in Phase 2, and the ledger and query cache in Phase 3.)*
- Compute every number in the shared query engine; the web UI renders those results and
  computes no authoritative totals of its own.

## Next Steps

This research is complete for initial design and feeds the
[urollup design](../../urollup-design.md), which owns the design and its decisions, and
the [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md), which owns
the implementation phases.
These items still need verification:

- [ ] Confirm the source-derived Codex and Pi behavior in running processes, starting
  with a Codex subagent’s `CODEX_THREAD_ID`, Codex hook input inside subagents, and Pi’s
  environment in `--no-session` runs.
- [ ] Resolve the open Codex details: whether `input_tokens` includes
  `cache_write_input_tokens`, usage semantics for non-OpenAI `model_provider` values,
  whether Desktop and IDE clients write paginated history, which release first wrote
  `info: null`, subagent replay before `rust-v0.152.0`, whether memory-consolidation
  threads link to a parent, how migration treats usage records line by line, and the
  MultiAgent V2 replay markers that ccusage documents but no code uses.
- [ ] Survey Codex signals in cloud environments, which this brief did not cover, and
  whether `CLAUDE_CODE_REMOTE_SESSION_ID` names a transcript file.
- [ ] Recheck Claude behavior known from third-party parsers or a small local sample on
  more versions: block-record `output_tokens` disagreement, `progress` records, `/btw`
  replays, the `subagents/workflows/` and `<session>/workflows/` layouts, whether Claude
  Code itself writes under `$XDG_CONFIG_HOME/claude`, entry-level `effort` (agentfdr
  claims effort appears only in command output), and the other undocumented fields.
- [ ] Check Pi’s `totalTokens` on captures from providers that report their own totals,
  and watch the experimental v4 session store and the `--mode rpc` shape.
- [ ] Run a small cloud export, download and local-merge smoke test in each cloud
  environment, covering log visibility, installation and network access, artifact
  retrieval and overlapping re-export, before advertising its compatibility.
- [ ] Check that public fixtures are synthetic or sanitized, with no prompt, path,
  identity or credential leakage.

## Methodology

The findings come from public source at pinned revisions, vendor documentation, package
metadata on GitHub and the npm registry, and key-name inspection of local logs.
On 2026-09-14, read-only partial clones of Codex, ccusage, Pi, agentfdr and the
Anthropic plugins were reviewed at the commits below with grep and targeted reads.
Nothing was executed except one synthetic probe, which compiled ccusage’s null-field
filter unchanged and ran it on two synthetic records, and no local logs were read for
that review.

- **Codex `rust-v0.154.0`, commit `6b9826e` (Apache-2.0):** The protocol, rollout
  recorder, file names, compression, archiving and migration, history, exec events and
  JSON output, session and subagent spawning, shell environment, hooks, SQLite state and
  their tests. Version bounds come from source snapshots at release tags between
  `rust-v0.100.0` and `rust-v0.153.0`, not from bisection.
- **ccusage 20.0.20, commit `bd7f89b` (tag `v20.0.20`, MIT):** The npm launcher, its six
  native platform packages, the Rust Claude, Codex, Pi and common adapters, the core
  pricing, cost, date, output and prefilter modules, the terminal and test-support
  crates, the benchmark generator and the Pi and Codex guides.
  The `v19.0.0` TypeScript source was read for comparison, and later `main` commits up
  to `95bbc41`, including `a4b8420`, `15b3bef` and `809eeb6`, as evidence of fixed bugs.
  *(Updated 2026-09-15: for the [ccusage feature inventory](#ccusage-feature-inventory),
  the same checkout’s CLI parser and its tests, command, configuration, output, cost and
  Codex report modules, npm package manifest and guide pages were read, the GitHub
  release list confirmed v20.0.20 as the latest release, the release notes and app
  directories of v18.0.11 and v19.0.0 dated the MCP package removal, and the commit log
  from v20.0.20 to `main` commit `a26f517` was reviewed for unreleased changes, and
  Claude Code’s status line documentation was read for the hook input fields.
  Nothing was executed.)*
- **Pi v0.85.1, commit `d981de1` (MIT):** The coding agent’s session manager, runtime,
  JSON mode, bash tool, usage totals, cache statistics, docs, changelog, tests and
  fixtures; the `pi-ai` message types, cost and provider usage mapping; and the agent
  loop and experimental v4 session store.
- **agentfdr 0.8.0, commit `e0904bf` (MIT):** All of `src/` except the i18n and HTML UI
  bodies, and all tests.
- **Anthropic plugins, commit `f0dce59` (Apache-2.0):** The session-report plugin in
  full, and the receipts plugin’s transcript miner and skill.
- **Metabrowser, commit `37011c4`:** The log adapters in `logutil/parsing.py`.
- **Claude Code and Anthropic API documentation:** Retrieved 2026-09-13; these pages are
  not versioned.
- **Local logs:** Key names only, from Claude Code 2.1-series transcripts, Codex Desktop
  0.15x rollouts and Pi 0.62.0 sessions, plus environment checks inside this research
  session’s Claude Code tool processes, all on 2026-09-13. No values, paths or IDs were
  copied.
- **LiteLLM and models.dev price data:** The snapshots that ccusage 20.0.20 pins in its
  `flake.lock` (commits `1a183ef` and `bff4122`), checked for effective-date fields.
- **fdu and flowmark-rs:** READMEs and build metadata at commits `afbb2ee` and
  `f1e9337`, as context for the Rust constraint.
- **softschema 0.8.1, commit `ff0f919` (tag `v0.8.1`):** The README and specification,
  for the summary data contract.

The synthetic example was checked against the cited parser code and tests, not against
session logs or provider invoices.
Source-derived facts describe code at the pinned commits, not observed runtime behavior,
and other log behaviors come from third-party parsers, vendor documentation, empirical
notes in the Anthropic plugins and a small local sample, so all can change between agent
versions.
No benchmarks were run for this brief, and it makes no universal performance or
billing-accuracy claim.
Cloud acquisition and persistent caching are proposed designs, not tested integrations.

## References

Tools and implementations, at the inspected revisions:

- [agentfdr 0.8.0 source](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90)
  (MIT):
  [Claude parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js),
  [Codex parser](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js),
  [subagent reader](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js),
  [anomaly detectors](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/detect.js),
  [usage view](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/usage.js)
  and
  [cost module](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/cost.js)
- [ccusage 20.0.20 source](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1)
  (MIT):
  [Claude adapter](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs),
  [Claude daily parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs),
  [Codex parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs),
  [Codex replay plan](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs),
  [Codex guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/codex/index.md)
  and
  [Pi guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/pi/index.md)
- ccusage commits after 20.0.20:
  [`a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84)
  (session-scoped Claude dedupe),
  [`15b3bef`](https://github.com/ccusage/ccusage/commit/15b3bef85b1e0d440ca98e345b4fb5610a41a195)
  (Codex cache writes) and
  [`809eeb6`](https://github.com/ccusage/ccusage/commit/809eeb6d52a2c7d13b9c65e10d4106109247390c)
  (Pi fork replay)
- ccusage 20.0.20 surface, for the [feature inventory](#ccusage-feature-inventory):
  [command tree](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-cli-parser/src/cli-commands.json),
  [CLI parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-cli-parser/src/parser.rs),
  [commands and status line](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs),
  [configuration](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-config/src/config.rs),
  [output](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/output.rs),
  [cost modes](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/cost.rs),
  [npm manifest](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/apps/ccusage/package.json)
  and
  [guide](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide)
- ccusage release notes:
  [v18.0.0](https://github.com/ccusage/ccusage/releases/tag/v18.0.0) (`blocks --live`
  removed), [v19.0.0](https://github.com/ccusage/ccusage/releases/tag/v19.0.0) (MCP
  package removed, commit
  [`d7e6993`](https://github.com/ccusage/ccusage/commit/d7e6993cc57852e693b0df86ab3904e3c118fa97))
  and [v20.0.20](https://github.com/ccusage/ccusage/releases/tag/v20.0.20)
- [ccusage `main` commit `a26f517`](https://github.com/ccusage/ccusage/tree/a26f5173fb425beb527b62f118e4275237368efa),
  the unreleased head reviewed on 2026-09-15
- [Claude Code status line documentation](https://code.claude.com/docs/en/statusline)
  (official docs, unversioned, retrieved 2026-09-15), for the status line comparison
- [ccusage 20.0.20 npm package](https://www.npmjs.com/package/ccusage/v/20.0.20)
- [ccusage 19.0.0 TypeScript source](https://github.com/ccusage/ccusage/tree/c5049cef6d830a6eef216534331ea3fa3da99314/apps/ccusage/src)
- [Anthropic session-report plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report)
  (official plugin, Apache-2.0; its transcript notes are empirical, not a format
  specification):
  [analyzer](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs)
- [Anthropic receipts plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts)
  (official plugin, Apache-2.0):
  [transcript miner](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs)
- [Metabrowser log adapters](https://github.com/jlevy/metabrowser/blob/37011c447cc4cad2d684045683648f641c172144/src/metabrowser/logutil/parsing.py)
- [fdu](https://github.com/jlevy/fdu/tree/afbb2eef01e94f37a4462549b0828ca8337a5f4c)
- [flowmark-rs](https://github.com/jlevy/flowmark-rs/tree/f1e9337e2d87ba614c231f0d17a2c181a3634117)
- [softschema 0.8.1](https://github.com/jlevy/softschema/tree/ff0f91999ebd0c3e8e0864e76042c6d875be083e):
  [specification](https://github.com/jlevy/softschema/blob/ff0f91999ebd0c3e8e0864e76042c6d875be083e/docs/softschema-spec.md)

Agent log formats:

- [Codex `rust-v0.154.0` source](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340)
  (Apache-2.0):
  [rollout recorder](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs),
  [compression](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs),
  [rollout line](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs),
  [exec events](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs),
  [exec JSON output](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs),
  [protocol](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs),
  [shell environment](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/shell_environment.rs),
  [exec environment](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs),
  [hooks schema](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs)
  and
  [hook runtime](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs)
- [Pi v0.85.1 source](https://github.com/earendil-works/pi/tree/d981de1229ef899957bbe968bc8dcda02a21f477)
  (MIT):
  [session format](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/session-format.md),
  [session manager](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts),
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

- [urollup design](../../urollup-design.md)
- [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md)
- [Rust CLI engineering baseline](research-2026-09-13-rust-cli-engineering-baseline.md)
- [Agent tool source reviews](research-2026-09-14-agent-tool-source-reviews.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
