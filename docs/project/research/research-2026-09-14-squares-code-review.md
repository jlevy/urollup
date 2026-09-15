---
title: squares Code Review for urollup
description: What urollup should port, test, share or avoid from the squares research repository's Claude Code and Codex usage rollups, session cost ledger and PR cost reporting, and what squares would need as a first urollup user.
date: 2026-09-14
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete for bead uro-o53w and reconciled with Codex source facts and the metaproc review (uro-ly32); recommendations are reflected in the urollup design or queued as review decisions in bead uro-gxen
---
# Research: squares Code Review for urollup

## Overview

[squares](https://github.com/jlevy/squares/tree/f2e24e07be8c94fa3ac603c3534dce7c454da99b)
is a square-packing research repository whose results are produced by long Claude Code
and Codex campaigns: about 127 recorded agent sessions, delegated lanes, subagent trees
and multi-hour autonomous runs.
Its operating rules require every terminal session to name what it cost and every pull
request to lead with what the branch cost
([OR-9](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/operating-rules.md#L249-L285)),
so it has grown its own usage tooling.

This review asks what urollup should reuse from that tooling and what squares would need
to adopt urollup. squares is relevant: about 3,900 lines of Python parse Claude Code
transcripts and Codex rollouts and tally tokens and time per log and per task tree,
another 2,300 lines retain privacy-reduced receipts in Git and join them to sessions,
branches and pull requests, and about 3,900 lines of tests surround them.
The code is Python, so reuse means porting logic, porting test cases and sharing format
ideas, never linking code.

Three results matter most:

- **Two counting defects:** squares sums Claude usage per transcript record and Codex
  usage per `token_count` event, with no request-level deduplication.
  Synthetic probes confirm both overcount (see
  [Counting Defects](#counting-defects-confirmed-with-synthetic-probes)). urollup’s
  reconciliation design already prevents them, and squares’ retained receipts must not
  serve as an oracle.
- **Codex lessons urollup lacks:** legacy subagent rollouts written before Codex marked
  the child boundary replay parent turns with no response IDs, and squares has tested
  heuristics that detect them when neither a start ordinal nor the parent rollout is
  available; it also has overlap-safe time definitions and append-stable cutoff
  semantics.
- **A concrete first user:** squares needs branch-attributed usage, interval receipts
  per session, completeness flags, non-additive shared attribution and structural
  command statistics, all of which fit urollup’s contracts with small additions.

## Questions to Answer

1. Which squares code for parsing Claude Code and Codex logs should urollup port, and
   which should it avoid?
2. How does squares tally tokens, time and cost per session, task tree, branch and
   experiment, and what lessons do its defects and history record?
3. Do squares’ receipt and ledger formats overlap urollup’s usage summary, and would
   squares consume urollup summaries?
4. Which filesystem, determinism and engineering practices are worth adopting?
5. Which fixtures and test cases can urollup reuse under the privacy constraints?

## Scope

Reviewed at commit `f2e24e07` (2026-09-13), the tip of `main` in the local checkout:

- **Code:** `packing/devtools/logrollup/`, `log_rollup.py`, `codex_log_rollup.py`,
  `codex_task_tree_delta.py`, `close_session.py`, `check_session_rollups.py`,
  `render_pr_rollup.py`, `render_wave_efficiency.py`, `validate_schemas.py` and
  `packing/src/sqpack/campaign/commit_clock.py`
- **Tests:** `test_log_rollup.py`, `test_codex_log_rollup.py`,
  `test_codex_task_tree_delta.py`, `test_codex_rollup_consumers.py` and
  `test_session_rollups.py`
- **Records and docs:** `README.md`, `AGENTS.md`, `operating-rules.md`,
  `development.md`, the campaign schemas, `packing/campaign/resource-usage/README.md`,
  `packing/campaign/agent-sessions/README.md`, usage-related entries in
  `packing/defects.yaml`, two usage reviews, and the commit history of the rollup tools

Excluded: the mathematics, the Rust search engine and the literature archive.
No local agent log content was read.
Counting behavior was checked by running squares’ readers on synthetic records written
to a scratch directory, plus one key-only count over recent local Codex rollouts.
Citations use `squares/<path>` with line ranges linked at the pinned commit.

**License and provenance:** squares publishes code as MIT and records as CC BY 4.0
([`squares/LICENSE:1-25`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/LICENSE#L1-L25)),
but it is the maintainer’s own repository, so urollup may port its code, tests and
documents freely; each ported item records the source repository (`jlevy/squares`) and
commit, per the design’s confirmed
[code reuse decision](../../urollup-design.md#decision-3-code-reuse-and-licensing).

**Related reviews:** the [metaproc review](research-2026-09-14-metaproc-code-review.md)
covers captured streams, harnesses, accounts and quotas, and holds the reconciliation
table between the two reviews.
This review holds the detailed treatment of Codex rollout parsing, subagent replay,
`token_count` handling and overlap-safe time measures.
Codex format facts cited here were checked against Codex source at
[`6b9826e`](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340)
(`rust-v0.154.0`).

## Findings

### How squares Measures Agent Work

squares keeps one enforced softschema receipt per measurement under
`packing/campaign/resource-usage/`: about 180 `ClaudeEfficiencyRollup/v1` records, one
per Claude transcript including subagent transcripts, and about 50
`CodexTaskTreeDelta/v1` interval receipts.
The receipt, not the log, is the retained artifact: the README cites a 29.9 MB
transcript reduced to a 23.0 KB record
([`squares/packing/campaign/resource-usage/README.md:3-25`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/resource-usage/README.md#L3-L25)).

The pipeline has four stages:

1. **Readers:** `devtools.log_rollup` detects a harness by content and writes one Claude
   record per log; `devtools.codex_log_rollup` scans a recursive Codex task tree under a
   root thread ID.
2. **Interval receipts:** `devtools.codex_task_tree_delta` subtracts two cutoff
   snapshots of a Codex tree into additive aggregates with prose, paths, child IDs and
   commands removed.
3. **Session join:** AgentSession records declare `resource_rollups`, and
   `check_session_rollups` refuses a terminal session with none
   ([`squares/packing/devtools/check_session_rollups.py:1-25`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/check_session_rollups.py#L1-L25)).
4. **Reports:** `close_session --render` writes a validated session-close report and a
   drift-checked `SYNOPSIS.md` table; `render_pr_rollup` prints the branch cost block
   that opens each pull request description.

squares reports tokens and time, never money: its throughput review labels its totals
usage rather than an invoice or dollar estimate
([`squares/docs/project/reviews/review-2026-09-07-research-throughput-and-timeboxes.md:16-24`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/docs/project/reviews/review-2026-09-07-research-throughput-and-timeboxes.md#L16-L24)),
and the Claude reader’s semantics decline to store a price because a retained price
would go stale without notice
([`squares/packing/devtools/logrollup/claude.py:318-322`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py#L318-L322)).
There is no subscription, budget-in-dollars or list-price versus actual-charge handling;
the metaproc review covers accounts, quota groups and plans.
Budgets are wall minutes and per-experiment `wall_seconds`, `agent_minutes` and
`pair_tests`, entered by agents
([`squares/packing/campaign/schemas/experiment.schema.yaml:154-181`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/schemas/experiment.schema.yaml#L154-L181)).

### Parsing Claude Code Transcripts

`ClaudeCodeReader` reads one transcript into a `SessionRollup`
([`squares/packing/devtools/logrollup/claude.py:183-271`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py#L183-L271)
and
[`404-486`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py#L404-L486)):

- **Detection:** by content, requiring `sessionId` and `uuid` in the first 40 lines, and
  a registry that refuses a file two readers claim rather than taking the first match
  ([`squares/packing/devtools/logrollup/reader.py:35-75`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/reader.py#L35-L75)).
- **Turns:** every `assistant` record becomes a turn with `message.model`, entry-level
  `effort`, `gitBranch`, `isSidechain`, the token counters, and a latency from the
  `parentUuid` record’s timestamp.
- **Tool calls:** `tool_use` blocks are paired with `tool_result` blocks by
  `tool_use_id`; the record keeps errors (`is_error`), denials (`toolDenialKind`),
  results with no seen call (normal after context compaction) and calls outstanding at
  the snapshot.
- **Session events:** counts of `system` records with subtypes `compact_boundary` (a
  context compaction) and `stop_hook_summary`, and `queue-operation` records, where an
  enqueue marks a user interjection during a turn.
- **Source identity:** SHA-256, bytes, record count, first `sessionId` and every
  distinct Claude Code `version`
  ([`squares/packing/devtools/logrollup/model.py:67-106`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/model.py#L67-L106)).

Lessons recorded in code and history:

- **Subagent transcripts carry the parent’s `sessionId`.** Naming records by session ID
  overwrote the parent’s record with the last subagent’s, so records are named by the
  log’s file stem
  ([`squares/packing/devtools/log_rollup.py:62-67`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/log_rollup.py#L62-L67);
  commit `cf380823`). The same commit found that subagents added 1,001 turns and 628
  tool calls that the parent transcript never recorded, and that finding subagent logs
  by hand missed some twice
  ([`squares/packing/devtools/close_session.py:1-18`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/close_session.py#L1-L18)).
- **Tool time is not tool execution.** The interval from `tool_use` to `tool_result`
  includes scheduling and permission waits, so it is neither execution time nor latency;
  both reviews call it a **tool interval**, resolving metaproc’s “tool latency” label.
  Claude Code records no model stream timing, so model time is unavailable rather than
  zero
  ([`squares/packing/devtools/logrollup/claude.py:289-322`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py#L289-L322)).
- **A command’s leading word is a poor identity.** In one session `cd` led 524 of 882
  commands, so `shell.py` splits chains, peels runners (`uv run`, `timeout 600`), flags,
  assignments, loop keywords and trailing filters, keeps subcommands for `git`, `gh`,
  `tbd` and project CLIs, and folds past 256 names into `(other)`
  ([`squares/packing/devtools/logrollup/shell.py:1-17`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/shell.py#L1-L17)
  and
  [`203-369`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/shell.py#L203-L369)).
  A structural shape (Python heredoc, inline Python, compound, pipeline) measures
  one-off scripting, the practice squares’ OR-1 discourages.

Weaknesses to avoid: `read_records` drops every malformed line, not only a pending tail,
and decodes with `errors="replace"`
([`squares/packing/devtools/logrollup/claude.py:130-146`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py#L130-L146));
the file is read three times into memory; and a missing `effort` becomes the string
`"None"`, a group indistinguishable from a real value.

### Parsing Codex Rollouts and Task Trees

`codex_log_rollup.build_rollup` produces `CodexEfficiencyRollup/v2` for one or more root
thread IDs
([`squares/packing/devtools/codex_log_rollup.py:1415-1547`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L1415-L1547)):

- **Discovery:** it reads the first line of every `*.jsonl` under a sessions root,
  requires a `session_meta` record, and keeps `id`, `parent_thread_id`, `agent_path`,
  `thread_source` and `subagent_history_start_ordinal`
  ([`427-514`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L427-L514)).
  Descendants are the transitive closure over `parent_thread_id` across all dates, and a
  cycle raises an error
  ([`1323-1330`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L1323-L1330),
  [`1436-1443`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L1436-L1443)).
  It does not read `.jsonl.zst` or the flat `archived_sessions/` directory, and when
  several files share a thread ID, as revert files do, the last sorted path silently
  wins.
- **Turns:** a task window runs from `task_started` to `task_complete`; a `task_started`
  with an open window closes the old one as interrupted, and a window open at the
  snapshot is live and ends at its last included event
  ([`628-657`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L628-L657),
  [`736-748`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L736-L748)).
  Model and effort come from the latest preceding `turn_context` for the turn; that
  model is the requested one, because Codex never saves the served model
  ([`codex-rs/core/src/session/turn.rs:2589-2601`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2589-L2601)).
- **Native timing:** `task_complete.duration_ms` and `time_to_first_token_ms` are kept
  with coverage counts, and negative or non-numeric values are unavailable rather than
  zero
  ([`184-190`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L184-L190),
  [`1004-1048`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L1004-L1048)).
- **Timed items:** `item_completed` events with `started_at_ms` and `completed_at_ms`
  give `Reasoning` and `AgentMessage` stream time and tool intervals by item type
  (`CommandExecution`, `McpToolCall`, `FileChange`, `CollabAgentToolCall`,
  `ContextCompaction`); older logs fall back to pairing `function_call` or
  `custom_tool_call` with its output by `call_id`, and correlate `write_stdin` polls
  with the command they poll
  ([`675-725`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L675-L725),
  [`839-891`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L839-L891)).
- **Context compaction:** current `ContextCompaction` items and legacy
  `context_compacted` events are counted without double counting when they coincide
  ([`764-772`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L764-L772)).

**Legacy subagent replay.** A subagent forked with its parent’s history into a legacy
(non-paginated) rollout copies the parent’s `session_meta`, `turn_context` and
`event_msg` records verbatim, including `task_started`, `task_complete` and
`token_count`, so the copies keep the parent’s turn IDs and the child’s running total
continues the parent’s
([`codex-rs/core/src/agent/control/spawn.rs:65-105`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105)).
The recorder stamps every copied line with its own write time
([`codex-rs/rollout/src/recorder.rs:1977-1994`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1977-L1994)),
which is why squares sees the replay at near-zero intervals after the child’s start.
The copied `token_count` events carry no response ID, and copies drop
`token_usage_record` entirely.
Codex marks the child’s own history in two ways: paginated children set
`subagent_history_start_ordinal`, and from `rust-v0.152.0` the first
`thread_settings_applied` whose `thread_id` names the child starts its history
([`codex-rs/protocol/src/protocol.rs:2183-2191`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2183-L2191)).
Parent turn IDs identify copies when the parent rollout is available.
Older files with none of these need heuristics, and squares removes the copies in two
layers:

1. **Prefix cut:** skip records up to the first `thread_settings_applied` after the last
   foreign `session_meta` (one whose `id` differs from the file’s thread), ignoring
   settings that a `compacted` record reapplies
   ([`454-495`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L454-L495)).
2. **Turn filter:** keep only turns that have their own `turn_context`, and drop a turn
   whose client `duration_ms` exceeds its local event interval by more than
   `max(1 s, 5%)`
   ([`221-229`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L221-L229),
   [`751-759`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L751-L759)).

History shows both heuristics were wrong once.
Commit `4ac71438` stopped treating a post-compaction `thread_settings_applied` as the
start of owned history, and commit `c59d28a7` made the duration check one-sided, because
an owned turn’s client duration excludes pauses and can be shorter than its interval.
Both fixes have tests
([`squares/packing/tests/test_codex_log_rollup.py:569-626`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_log_rollup.py#L569-L626)
and
[`757-1013`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_log_rollup.py#L757-L1013)).

**Local Codex state.** The PR 156 usage audit read metadata columns from Codex’s local
SQLite stores, `state_5.sqlite` (`threads`, `thread_spawn_edges`) and
`thread_history_1.sqlite` (`thread_turns`), to confirm parent links and terminal turn
status; the rollouts still marked two trees incomplete that the thread index showed as
finished
([`squares/docs/project/reviews/review-2026-09-13-pr156-usage-delta.md:90-126`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/docs/project/reviews/review-2026-09-13-pr156-usage-delta.md#L90-L126)).
In Codex source these tables mirror rollouts, and for paginated threads they are
authoritative for the current rollout path, so they are an optional discovery hint and
never a usage source
([`codex-rs/state/src/lib.rs:1-5`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/lib.rs#L1-L5),
[`codex-rs/thread-store/src/local/thread_rollout_resolver.rs:91-94`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/thread_rollout_resolver.rs#L91-L94)).

### Counting Defects Confirmed With Synthetic Probes

Both readers were run on synthetic records written to a scratch directory; no real log
was used.

| Probe | Input | squares output | Reconciled value |
| --- | --- | --- | --- |
| Claude content blocks | One response written as 3 `assistant` records sharing `message.id` and `requestId`, each with usage 3 input, 40,000 cache read, 600 output | 3 turns; 9 input, 120,000 cache read, 1,800 output | 1 request; 3, 40,000 and 600 |
| Codex repeated snapshot | One task with two identical consecutive `token_count` events (100 input, 20 output) and one `info: null` rate-limit event | 3 model responses; 200 input, 40 output | 1 response; 100 and 20 |

- **Claude:** `parse` appends a turn per `assistant` record and `read` sums every turn’s
  counters
  ([`squares/packing/devtools/logrollup/claude.py:192-213`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py#L192-L213),
  [`411-415`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py#L411-L415)).
  This is the content-block case in the synthetic double-counting example of the
  [portable research brief](research-2026-09-13-portable-agent-usage.md).
  `turns.assistant` counts records, not requests, and squares’ PR cost block prorates
  branch cost by it.
- **Codex:** every `token_count` event inside a task window adds its `last_token_usage`
  and increments `model_response_count`, even when `info` is null
  ([`squares/packing/devtools/codex_log_rollup.py:278-290`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L278-L290),
  [`942-956`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L942-L956)).
  A key-only count found `info: null` token events in 2 of 200 recent local rollouts.
  The scanner does not read `token_usage_record`.

Codex source explains each input the scanner mishandles
([`codex-rs/core/src/session/mod.rs:4447-4501`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4447-L4501),
[`codex-rs/core/src/session/turn.rs:1484-1494`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L1484-L1494)):

- **Repeated snapshots:** rate-limit updates re-emit `token_count` with unchanged
  `info`, so identical consecutive totals are one observation.
- **`info: null`:** the thread has no recorded usage yet; the event is a rate-limit
  observation, never a request.
- **Synthetic values:** a context compaction emits an estimate whose `last_token_usage`
  has only `total_tokens`, and a full context window rewrites the running total to the
  window size with input and output reset to zero, so later cumulative components
  restart.
- **Per-response records:** from `rust-v0.153.0`, `token_usage_record` holds one
  response’s usage with its `response_id`, `thread_id` and turn IDs; a response without
  usage writes none, a record whose `thread_id` differs from the file’s thread is a
  copy, and so is `compacted.latest_token_usage_record`
  ([`codex-rs/protocol/src/protocol.rs:2237-2248`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2237-L2248)).

The size of the error in squares’ retained receipts is unknown and cannot be measured
from the receipts, which hold no request identities.
None of squares’ tests covers either case: the Codex fixture builder emits one
`token_count` per response
([`squares/packing/tests/test_codex_log_rollup.py:102-122`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_log_rollup.py#L102-L122)).

### Tallying Time and Parallel Work

squares’ Codex time model is more detailed than urollup’s time measure contract and has
been used in several efficiency reviews:

| Measure | Definition in squares | Bound |
| --- | --- | --- |
| Active seconds | Union of a thread’s task windows | Exact for recorded windows |
| Wall span and inactive gap | First task start to last task end, minus active time | Upper bound on work |
| Agent-active seconds | Sum of active seconds over a subtree | Counts parallel agents separately |
| Active union and parallel overlap | Union of subtree windows; agent-active minus union | Exposes concurrency without double counting |
| Tool seconds by category | Explicit tool intervals clipped to active windows, each instant assigned to one category by priority (`agent_wait`, `command`, `mcp`, `agent_control`, `file_change`, `extension`, `compaction`, meaning context compaction) | Overlap-safe |
| Response envelope | Active time minus explicit tool and context compaction intervals | Upper bound on model time |
| Timed model stream | `Reasoning` and `AgentMessage` item durations | Lower bound; absent in legacy logs |
| First-token wait | Sum of `time_to_first_token_ms` on completed turns | First response of each turn only |

Sources:
[`squares/packing/devtools/codex_log_rollup.py:193-261`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L193-L261),
[`1162-1217`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L1162-L1217),
[`1338-1357`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L1338-L1357)
and the embedded semantics at
[`1483-1545`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L1483-L1545).
urollup’s [measure contracts](../../urollup-design.md#41-measure-contracts) cover span,
summed durations, busy union, waiting and critical path, but not agent-seconds versus
union, overlap-safe tool categories, or upper and lower model-time bounds.
The metaproc review compares these definitions with metaproc’s nested sums and qm’s
per-phase gap decomposition; the reconciled position keeps squares’ definitions and adds
qm’s explicit residual for unattributed time.

### Intervals, Cutoffs and Delta Receipts

`codex_task_tree_delta.build_delta` builds two retrospective rollups at `--start` and
`--end`, subtracts a whitelist of additive fields, and rejects any decrease
([`squares/packing/devtools/codex_task_tree_delta.py:574-578`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_task_tree_delta.py#L574-L578),
[`638-723`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_task_tree_delta.py#L638-L723)).
Its semantics are explicit:

- **Clipping:** task, tool, context compaction and stream intervals are clipped at both
  cutoffs, but only when a later record proves they straddled the cutoff
  ([`squares/packing/devtools/codex_log_rollup.py:577-607`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L577-L607)).
- **Completion attribution:** token counts and first-token waits are emitted on
  completion and charged wholly to the interval containing the completion.
- **Live lower bounds:** a receipt whose end snapshot contains an open turn is marked
  `snapshot_incomplete` with a count of live sessions, and the PR cost renderer labels
  it a lower bound.
- **Append stability:** a receipt for a fixed cutoff is unchanged after later records,
  including a context compaction, are appended to the log
  ([`squares/packing/tests/test_codex_log_rollup.py:916-1013`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_log_rollup.py#L916-L1013)).
- **Private validation errors:** the semantic validator bounds its problem list at 64
  and never echoes values, which a test enforces
  ([`squares/packing/devtools/codex_task_tree_delta.py:366-454`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_task_tree_delta.py#L366-L454);
  [`squares/packing/tests/test_codex_task_tree_delta.py:542-556`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_task_tree_delta.py#L542-L556)).

urollup’s `--since` and `--until` clip usage to a half-open interval by a declared
response-timestamp policy, which matches completion attribution for tokens.
Subtracting cumulative snapshots is unnecessary in urollup, because the ledger holds
request rows; the lessons are the clipping rule for intervals, the live-lower-bound flag
and the append-stability test.

### Attribution to Sessions, Branches and Pull Requests

**Shared logs are the main overcount in squares’ own history.** Several sessions often
declare the same long-running coordinator log.
Adding per-session figures reported 117.9 hours for a campaign that had spent 43.7, so
every total is now over distinct receipts, a receipt declared by several sessions is
charged to none and shown on its own row, and receipts no session declares are counted
as unattributed rather than assigned by span containment
([`squares/packing/campaign/schemas/session-close-report.schema.yaml:24-37`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/schemas/session-close-report.schema.yaml#L24-L37);
[`squares/packing/devtools/close_session.py:596-620`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/close_session.py#L596-L620)).
Sessions with no receipt appear with `measured: false` and a reason, and a stopped
session whose logs are gone declares `resource_usage_unmeasured` with reason
`native_harness_data_unavailable`
([`squares/packing/campaign/agent-sessions/README.md:200-217`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/agent-sessions/README.md#L200-L217)).

**Branch cost is a bound, not a figure.** Claude records `gitBranch` on every record,
but the rollup keeps only `turns.by_branch`, so tokens and tool calls cannot be split
for a log that straddles branches.
`render_pr_rollup` therefore prints three columns: logs entirely on the branch, all logs
prorated by turn share, and every log that touched the branch
([`squares/packing/devtools/render_pr_rollup.py:1-37`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/render_pr_rollup.py#L1-L37),
[`191-254`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/render_pr_rollup.py#L191-L254)).
On the branch that introduced OR-9, straddling logs held 5,486 of 8,423 turns.
Codex records the branch once per thread in `session_meta.git.branch`, beside
`commit_hash` and `repository_url`
([`codex-rs/protocol/src/protocol.rs:3334-3348`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3334-L3348)),
but squares’ scanner does not read it, so an AgentSession must declare the branch, the
renderer labels the association operator-recorded, and one receipt claimed for two
branches is refused
([`squares/packing/tests/test_codex_rollup_consumers.py:138-252`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_rollup_consumers.py#L138-L252)).

**Cross-harness totals are refused.** Claude and Codex receipts render in separate
tables because their units differ and their measurements can overlap, and the wave
renderer refused a mixed Claude-and-Codex agenda outright; the outstanding defect D-421
calls for a shared additive measurement that keeps each harness’s completeness and
overlap semantics
([`squares/packing/defects.yaml:12614-12638`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/defects.yaml#L12614-L12638)).
squares planned a unified `EfficiencyRollup` for that purpose and never built it
([`squares/packing/campaign/resource-usage/README.md:42-47`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/resource-usage/README.md#L42-L47)).

### Formats and Contracts

- **Receipts:** pure-YAML softschema artifacts with `softschema.contract`, a relative
  `schema` pointer, `envelope: rollup` and `status: enforced`
  ([`squares/packing/devtools/logrollup/model.py:176-210`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/model.py#L176-L210)).
  squares’ gate requires the `schema` pointer and validates each receipt with
  `jsonschema_rs` against the file it names, so closure must be compiled into the schema
  ([`squares/packing/devtools/validate_schemas.py:114-136`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/validate_schemas.py#L114-L136),
  [`301-308`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/validate_schemas.py#L301-L308)).
  A standalone urollup summary omits `schema`, so squares could not commit one to that
  directory unchanged.
- **Required semantics:** every receipt carries a `semantics` map from figure to
  meaning, and the writer refuses an empty one, because receipts outlive the tools and
  logs that explain them
  ([`squares/packing/campaign/resource-usage/README.md:70-83`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/resource-usage/README.md#L70-L83)).
- **Receipt references:** sessions cite receipts by exact repository-relative path;
  absolute, traversal, basename-only and nested references are refused
  ([`squares/development.md:1145-1165`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/development.md#L1145-L1165)).
- **Regenerated, not appended:** a Claude receipt is a function of its log and a Codex
  receipt of its root and two cutoffs, so rerunning replaces it
  ([`squares/packing/campaign/agent-sessions/README.md:295-299`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/agent-sessions/README.md#L295-L299)).

The overlap with urollup’s
[usage summary](../../urollup-design.md#52-usage-summary-format) is substantial: both
are enforced pure-YAML softschema artifacts that exclude content, identify their
sources, and keep unknown values distinct from zero.
urollup’s summary is strictly more capable for squares’ needs, because its extents merge
without double counting a log that several sessions declare.

### Engineering Practices

- **Verdicts anchored to the commit:** four ledger refusals compared deadlines with
  `now()`, so a green commit turned red between CI runs; the fix anchors them to HEAD’s
  committer date and reports them uncheckable outside a checkout (D-468,
  [`squares/packing/src/sqpack/campaign/commit_clock.py:1-30`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/src/sqpack/campaign/commit_clock.py#L1-L30)).
- **Agents misread their own clocks:** a coordinator estimated elapsed time between tool
  calls, recorded blocks of 150, 180, 180 and 40 minutes that commit timestamps show
  took 31, 42, 29 and 23, and stopped with most of its budget unspent under a false
  reason; the fix was a gate that refuses start times later than the check (D-358 and
  D-386,
  [`squares/packing/defects.yaml:9504-9549`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/defects.yaml#L9504-L9549)).
- **Numbers cite their receipt:** an operating rule quoted figures read by hand from a
  transcript that disagreed with the retained rollup, in the flattering direction
  (D-379,
  [`squares/packing/defects.yaml:10577-10620`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/defects.yaml#L10577-L10620)).
- **Generated Markdown owned by one formatter:** `close_session` emits tables only into
  `SYNOPSIS.md`, because flowmark would rewrap generated prose and the drift check would
  then demand it back
  ([`squares/packing/devtools/close_session.py:536-551`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/close_session.py#L536-L551)).
- **Hashes name their function:** Git is the integrity boundary for committed files, a
  checksum is justified only across a real trust boundary, and content identities for
  deduplication or caching must say so
  ([`squares/development.md:905-917`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/development.md#L905-L917)).
- **Stale caches under same-size edits:** validation commands get an empty
  bytecode-cache root because rapid same-size source mutations executed stale code
  ([`squares/development.md:515-516`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/development.md#L515-L516)).
- **Negative controls:** each guard has a mutation in `devtools/controls.yaml` proving
  it fires, the same idea as urollup’s gate-failure proofs.
- **Atomic writes:** the delta CLI publishes through `strif.atomic_output_file`, but
  `log_rollup` writes records in place
  ([`squares/packing/devtools/log_rollup.py:66-67`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/log_rollup.py#L66-L67)).
  urollup’s tempfile-and-persist rules already exceed this.

### Fixtures and Tests

- **Portable test designs:** the Codex tests build synthetic rollout records in Python
  with placeholder UUIDs, `/workspace` as `cwd` and fake model names, covering tool
  subtraction and model grouping, recursive trees, interrupted and live turns, explicit
  cutoffs, invalid native timing, context compaction double counting, legacy replay with
  and without foreign metadata, and append stability
  ([`squares/packing/tests/test_codex_log_rollup.py:15-1013`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_log_rollup.py#L15-L1013)).
  The shell resolver tests cover peeling, keywords, filters by position, the
  trailing-slash crash and long-tail folding
  ([`squares/packing/tests/test_log_rollup.py:16-95`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_log_rollup.py#L16-L95)).
- **Gaps:** no fixture has Claude content-block duplication, resumed or forked Claude
  history, Codex `token_usage_record`, repeated snapshots, synthetic estimates or
  context-window resets, `forked_from_id`, `.zst` rollouts, multiple files per thread or
  rate limits.
- **Retained receipts are not fixtures:** they are aggregates with no request rows,
  carry real session and agent IDs and model names, and inherit the counting defects
  above.

## Reuse Table

Reuse modes: **port logic** (reimplement in Rust from the Python), **port tests**
(rewrite the cases as urollup fixtures and goldens), **share format** (adopt a format
idea or coordinate a contract), **learn** (a lesson with no code), **avoid** (a behavior
not to copy). The findings above link each line range at the pinned commit.

| Item | Where in squares | Reuse mode | urollup bead or plan section | Notes and risks |
| --- | --- | --- | --- | --- |
| Legacy Codex subagent replay detection | `squares/packing/devtools/codex_log_rollup.py:221-229`, `454-495`, `751-759`; `squares/packing/tests/test_codex_log_rollup.py:569-626`, `757-1013` | Port logic and port tests | uro-y2qj; uro-spce; design [Reconciliation](../../urollup-design.md#33-reconciliation) | Fallback only, after the start ordinal, the child’s own `thread_settings_applied` and parent turn IDs; the heuristics changed twice, so label exclusions `inferred` with diagnostics |
| Codex turn windows, interrupted and live states | `squares/packing/devtools/codex_log_rollup.py:628-657`, `736-748`; `squares/packing/tests/test_codex_log_rollup.py:412-499` | Port logic and port tests | uro-y2qj; design Measure Contracts (Time) | A live turn at snapshot is a completeness fact the summary does not yet carry |
| Native Codex timing with coverage | `squares/packing/devtools/codex_log_rollup.py:184-190`, `1004-1048`; `squares/packing/tests/test_codex_log_rollup.py:628-708` | Port logic | uro-y2qj (capture native fields) | Keep `duration_ms` and `time_to_first_token_ms` verbatim as observed fields |
| Overlap-safe time measures | `squares/packing/devtools/codex_log_rollup.py:193-261`, `1162-1217`, `1338-1357` | Port logic (definitions) | uro-spce; uro-d135; design Measure Contracts; summary `busy` | Category priority is a policy choice; version it |
| Cutoff clipping and append stability | `squares/packing/devtools/codex_log_rollup.py:577-607`; `squares/packing/devtools/codex_task_tree_delta.py:638-723`; `squares/packing/tests/test_codex_log_rollup.py:916-1013` | Port tests; learn | uro-d135 (`--since`, `--until`); uro-unib (extent stability) | Tokens by completion time, intervals clipped only when proven to straddle |
| Content detection, ambiguous claim refused | `squares/packing/devtools/logrollup/reader.py:35-84` | Port logic | uro-y2qj (dialect detection) | Matches urollup’s design; state the refusal explicitly |
| Claude tool pairing and session events | `squares/packing/devtools/logrollup/claude.py:183-271`, `462-476` | Port logic | uro-y2qj; design [Capture Layers](../../urollup-design.md#23-capture-layers-and-re-extraction) | `queue-operation` and `system` subtypes may fall outside the capture rule |
| Claude token summation per record | `squares/packing/devtools/logrollup/claude.py:192-213`, `411-415` | Avoid | uro-spce; uro-obx5 | Confirmed overcount on a synthetic probe |
| Codex usage per `token_count` event | `squares/packing/devtools/codex_log_rollup.py:278-290`, `942-956` | Avoid | uro-spce; uro-y2qj | Overcounts repeated snapshots and counts `info: null` as a response |
| Codex discovery | `squares/packing/devtools/codex_log_rollup.py:427-514`, `1436-1443` | Learn | uro-20ck | Misses `.zst`, `archived_sessions/` and multi-file threads; a cycle aborts the run |
| Shell command resolver and shapes | `squares/packing/devtools/logrollup/shell.py:203-369`; `squares/packing/devtools/logrollup/claude.py:49-105`; `squares/packing/tests/test_log_rollup.py:16-95` | Port logic and port tests | uro-d135 (`tools`); design Capture Layers (strip policy) | Must run before arguments are stripped; project CLI families need configuration |
| Required `semantics` block; unavailable is not zero | `squares/packing/devtools/logrollup/model.py:176-210`; `squares/packing/devtools/logrollup/claude.py:289-378` | Share format | uro-unib; uro-d135 | urollup could carry definition IDs rather than prose per file |
| Distinct-receipt totals and shared rows | `squares/packing/devtools/close_session.py:374-470`, `596-620`; `squares/packing/campaign/schemas/session-close-report.schema.yaml:24-37` | Learn; integration target | design [Ownership and Totals](../../urollup-design.md#42-ownership-and-totals); uro-52qi | Real-world evidence for non-additive tag groups |
| Branch cost bounds | `squares/packing/devtools/render_pr_rollup.py:191-254`; `squares/operating-rules.md:249-285` | Learn; integration need | uro-d135 (`--group-by branch`) | urollup can make Claude branch cost exact from per-record `gitBranch`, and observe Codex branch per thread from `session_meta.git.branch` |
| Delta validator that never echoes values | `squares/packing/devtools/codex_task_tree_delta.py:366-454`; `squares/packing/tests/test_codex_task_tree_delta.py:542-556` | Port tests | uro-unib (`validate`) | Applies to diagnostics about redacted fields |
| Synthetic Codex record builders | `squares/packing/tests/test_codex_log_rollup.py:15-122` | Port tests | uro-obx5 | No private data; record the source commit; extend for the gaps listed above |
| Retained receipts corpus | `squares/packing/campaign/resource-usage/*.yaml` | Avoid as fixtures; learn | uro-o65n | Real IDs, overcounted; at most a labeled third comparator |
| Codex SQLite thread index | `squares/docs/project/reviews/review-2026-09-13-pr156-usage-delta.md:117-139` | Learn | uro-20ck | Undocumented and versioned by file name; a discovery hint, never a usage source |
| Commit-anchored verdicts | `squares/packing/src/sqpack/campaign/commit_clock.py:1-30`; `squares/packing/defects.yaml:15241-15305` | Learn | uro-jpmf (`check`); plan QuerySpec | Resolve relative times to absolute instants in saved queries |
| Generated tables and flowmark | `squares/packing/devtools/close_session.py:536-551` | Learn | uro-d135 (Markdown goldens) | Markdown output should be a fixed point of pinned flowmark |
| Hash policy | `squares/development.md:905-917` | Learn | design Analytical Identities; Capture Cache | Name each digest’s function: identity, cache check or trust boundary |

## Key Insights

1. **squares’ receipts disagree with urollup by design, and the difference will look
   like a regression.** Moving squares to urollup will lower Claude token totals by the
   content-block factor and Codex totals by repeated snapshots.
   The feature matrix and any migration note must explain the drop from source records
   before squares retires its readers, or its maintainers will read urollup as
   undercounting.
2. **Legacy Codex subagent replay defeats identity-based deduplication.** A fallback
   request key for an event with no response ID that includes the containing thread
   counts replayed parent `token_count` events in a legacy child twice.
   The child boundary therefore needs an ordered rule: `subagent_history_start_ordinal`,
   else the first `thread_settings_applied` naming the child (0.152+), else parent turn
   IDs when the parent rollout is available, else heuristics labeled `inferred`. squares
   supplies the tested heuristics for that last tier and shows they need care.
3. **Stripping arguments removes the most-used tool analysis.** squares’ most-read
   tables (command identity, command family, one-off scripting shapes) come from shell
   command text. urollup’s strip policy replaces tool arguments with length and digest
   stubs, so a later `tools` report cannot recover command names from captured records.
   A versioned structural command summary must be captured before stripping.
4. **Completeness has three states, not two.** A Codex turn can be complete, open at the
   snapshot (a live lower bound), or abandoned with no `task_complete` even though the
   thread index shows terminal work.
   squares excluded whole trees for the last case.
   urollup has `nonfinal` usage revisions and snapshot cutoffs, but no per-thread
   open-turn flag.
5. **Branch is the unit squares bills against.** Claude’s per-record `gitBranch` makes
   branch attribution observed and exact at request level, which squares’ per-log
   receipts cannot do. Codex branch attribution is observed per thread from
   `session_meta.git.branch`, so it is exact only for threads that stay on one branch;
   squares’ operator-recorded label remains the fallback where a rollout records none.
6. **Tag groups overlap in practice.** squares’ 117.9-versus-43.7-hour error is the
   non-additive tag case the urollup contracts name in one sentence.
   Merged summaries remove the duplicate log automatically, but reports grouped by an
   annotation that several sessions share must still label rows non-additive and show
   the shared remainder.
7. **Model time has bounds, not a value.** Codex logs support an upper bound (response
   envelope) and a lower bound (timed stream) plus first-token wait; Claude transcripts
   support neither, and a `claude-stream` result’s `duration_api_ms` covers a whole
   invocation (see the metaproc review).
   A single “model seconds” measure would be wrong for both.
8. **Agents need measured clocks.** Two squares defects came from agents estimating
   elapsed time. A fast `urollup report --current` that states span and busy time is a
   direct remedy, which raises the priority of current-session latency on large
   transcripts.
9. **Money is optional for this user.** squares never prices usage, and some of the
   model names in its receipts may not match a reviewed price table.
   Every urollup report must be complete and useful with pricing absent, and
   `--require-priced` must stay opt-in.
10. **Guardian reviews are a large hidden cost.** In the PR 156 audit, automatic
    approval reviews inside the recursive trees accounted for 289 of 2,191 responses and
    about 8% of input tokens
    ([`squares/docs/project/reviews/review-2026-09-13-pr156-usage-delta.md:26-33`](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/docs/project/reviews/review-2026-09-13-pr156-usage-delta.md#L26-L33)).
    In Codex source a guardian trunk review is a child session with `parent_thread_id`
    and `thread_source: guardian_review`, so `--scope descendants` includes it, while
    parallel guardian reviews run as ephemeral forks whose usage never reaches disk
    ([`codex-rs/core/src/codex_delegate.rs:82-116`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/codex_delegate.rs#L82-L116),
    [`codex-rs/core/src/guardian/review_session.rs:787-797`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/guardian/review_session.rs#L787-L797)).

## Integration Needs

What squares would need from urollup to replace its readers:

- **Per-session receipts from a selection and interval:**
  `urollup export --session <root> --scope descendants --since <start> --until <end>
  --format summary`, with Claude subagent transcripts found automatically, replacing
  both `close_session --update --agent-logs` and `codex_task_tree_delta`.
- **Branch grouping:** `--group-by branch` from Claude `gitBranch` per request and Codex
  `session_meta.git.branch` per thread as observed properties, and a configured branch
  from a manifest or annotation set, labeled configured, where no branch is recorded.
- **Completeness:** per-extent flags for open turns at the snapshot and abandoned turns,
  so squares can mark lower bounds or exclude trees without a separate scanner.
- **Distinct totals with attribution:** summary merge for campaign totals, and an
  annotation set mapping squares session IDs to threads or intervals, with shared
  extents reported as a non-additive remainder and unattributed extents counted
  separately.
- **Time measures:** agent-active seconds, busy union, parallel overlap, tool intervals
  by category, context compaction time, and model-time upper and lower bounds with
  first-token wait, all with explicit availability per dialect.
- **Tool statistics:** tool call counts, errors, denials, unpaired results and
  structural command identities with configurable project families, captured before
  stripping.
- **Committable summaries:** a way to write `softschema.schema` as a repository-relative
  pointer next to a copied compiled schema, deterministic bytes for drift checks, and
  Markdown that is stable under flowmark.
- **Installation:** a PyPI binary wheel squares can pin in its dev group under a release
  cool-off, since squares runs everything through `uv run --frozen`.
- **Retention warning:** `sources` output that shows sessions nearing Claude Code’s
  `cleanupPeriodDays`, so receipts are exported before logs disappear; squares already
  needs an explicit unmeasured state for this.
- **Experiment rounds:** measured agent time and tokens for a time window and session
  selection, to replace hand-entered `agent_minutes` in experiment records and the
  agent-minutes versus machine-minutes check in the experiment-loop skill.

squares would not need the web UI, pricing, cloud bundles or the persistent cache to
start.

## Recommendations

Changes for the maintainer to consider; this review does not edit those documents.
Items marked *(Updated)* were revised on 2026-09-14 against Codex source; the
[metaproc review](research-2026-09-14-metaproc-code-review.md) recommends the
captured-stream, account and quota changes and cites these recommendations by number.

*(Updated 2026-09-15: the architecture doc and plan sections these groups target were
replaced by the [urollup design](../../urollup-design.md).
Recommendations 1 to 3 and 9 are reflected there, and 11, 13 and 14 in part.
Recommendations 4 to 8, 10, 12 and 19 are queued review decisions in
[design §9.2](../../urollup-design.md#92-queued-review-decisions), bead `uro-gxen`.)*

**Architecture doc (now the design doc), reconciliation and capture:**

1. *(Updated: replay facts settled by source.)* Add an ordered child-boundary rule for
   Codex child rollouts: `subagent_history_start_ordinal`; else the first
   `thread_settings_applied` whose `thread_id` is the child’s (0.152+); else, when the
   parent rollout is available, treat records whose turn IDs appear in it as copies,
   because replays keep parent turn IDs; else squares’ fallback checks (no own
   `turn_context`; client `duration_ms` above the interval by more than `max(1 s, 5%)`;
   ignore post-compaction settings), marking excluded usage `inferred` with a
   diagnostic. Never bucket copied records by their line timestamps, which are write
   times. Track the fixtures in uro-y2qj and uro-spce.
2. *(Updated: semantics settled by source.)* State in the Codex adapter contract that a
   `token_count` event with null `info` is a provider limit observation only, never a
   request; that consecutive identical cumulative totals contribute nothing; that a
   `last_token_usage` with only `total_tokens` is a context compaction estimate reported
   as a diagnostic; that a decrease in any cumulative component opens a new counter
   epoch; and that `token_usage_record`, where present (0.153+), replaces `token_count`
   differencing.
3. Extend captured records to keep session lifecycle records: Claude `system` records
   with `compact_boundary` and `stop_hook_summary` subtypes, `queue-operation` records,
   tool denials and `is_error` flags, and Codex `task_started`, `task_complete` and
   timed `item_completed` records.
4. Add a structural command summary to the strip policy: resolved invocation names with
   runners and subcommands, shape and traits, computed by a versioned classifier before
   arguments are stubbed.
   Port squares’ resolver rules and tests, with project families supplied by
   configuration.
5. *(Updated: terminology.)* Extend the time measure contract with agent-active seconds
   versus busy union and parallel overlap, overlap-safe tool intervals by category with
   a versioned priority, context compaction time, and model-time bounds (response
   envelope, timed stream, first-token wait) with per-dialect availability.
   Name the `tool_use` to `tool_result` measure a tool interval, as metaproc
   recommendation C2 also does.
6. Add a per-extent completeness field for open and abandoned turns at the snapshot, and
   make `--strict` treat open turns as a coverage gap.

**Plan and contracts (now the design doc), selection and output:**

7. *(Updated: Codex records the branch.)* Add `branch` as an observed request property
   from Claude `gitBranch`, an observed thread property from Codex
   `session_meta.git.branch`, and a configured property elsewhere, with
   `--group-by branch`, to uro-d135.
8. Capture Codex `agent_path`, `agent_role` and `agent_nickname` as thread properties
   and allow grouping by them; squares selects and names lanes by `agent_path`.
9. *(Updated: answered by source.)* State in the relationship rules that a guardian
   trunk review is a parent-linked child included by `--scope descendants`, and report
   parallel guardian reviews as unobserved usage, never zero; add a fixture for the
   first case.
10. Add an export option to write a relative `softschema.schema` pointer beside a copied
    compiled schema, for repositories whose gates validate by pointer.
11. Resolve relative time expressions to absolute instants in every normalized
    `QuerySpec`, and keep `check` verdicts independent of the wall clock.
12. Add a golden test that Markdown reports are unchanged by the pinned flowmark.
13. Add a `sources` diagnostic for sessions within a configurable number of days of
    Claude Code’s transcript cleanup.
14. Document each digest’s function (identity, cache check, trust boundary), and note
    that the capture cache’s default prefix check misses a same-size mutation inside the
    captured extent that leaves both digests unchanged, which `--verify-cache` detects.

**Beads:**

15. *(Updated: provenance, and cases from source.)* uro-obx5: port squares’ synthetic
    Codex builders and case list, recording the squares repository and commit, and add
    the missing cases (content blocks, repeated snapshots, null `info`, context
    compaction estimates, context-window resets, `token_usage_record`, forks, `.zst`,
    multi-file threads, legacy replay with parent turn IDs and rewritten timestamps).
16. uro-o65n: add squares’ readers as a labeled third comparator for token and time
    figures, explaining each disagreement from source records.
17. *(Updated: confirmed by source.)* uro-20ck: use Codex’s local SQLite thread index as
    an optional, version-gated discovery and turn-status hint, never a usage source.
18. uro-unib: add a test that validation diagnostics never echo values from redacted
    fields.
19. Create a bead for squares as the first integration user, blocked on Phase 1, that
    replaces `log_rollup`, `codex_task_tree_delta` and the session-close sums with
    urollup summaries and records the expected drop in totals.

## Next Steps

- [ ] Maintainer review of the recommendations above, alongside the metaproc review
  (uro-ankt). *(Updated 2026-09-15: the queued recommendations continue in bead
  `uro-gxen`, as noted under [Recommendations](#recommendations).)*
- [x] Verify against Codex source what `info: null` token events mean and whether legacy
  subagent replays keep their parent turn IDs: at `rust-v0.154.0`, null `info` precedes
  a thread’s first recorded usage and replays keep parent turn IDs; the first release
  writing null `info` and pre-0.152 replay details remain unverified.
- [ ] After urollup’s Claude and Codex adapters exist, measure the overcount in squares’
  receipts on consented local logs, recording ratios only.

## Methodology

- Read squares at `f2e24e07` with `git grep`, full reads of the rollup modules, tests,
  schemas and campaign READMEs, and targeted reads of reviews, defects and commit
  messages for the rollup tools (`git log -- packing/devtools/logrollup
  packing/devtools/codex_log_rollup.py packing/devtools/codex_task_tree_delta.py`).
- Ran `ClaudeCodeReader.read` and `build_rollup` from squares’ own virtual environment
  on synthetic records written to a scratch directory, with bytecode writing disabled so
  the checkout was not modified.
- Counted recent local Codex rollouts containing `"type":"token_count","info":null` by
  key match only, printing two counts and no content, paths or IDs.
- Compared findings with the urollup plan, data contracts and portable research brief as
  they stood on 2026-09-14.

The squares figures quoted here (hours, turn counts, response shares) come from its
public records, not from local logs.
Heuristic details for legacy Codex replay come from squares’ code, tests and commit
history; the Codex format facts (replay copies, `token_count` semantics,
`token_usage_record`, branch, guardian reviews, SQLite stores) come from a read-only
source review of Codex at `6b9826e` (`rust-v0.154.0`), not from a running process.

## References

squares at commit `f2e24e07`:

- [Repository](https://github.com/jlevy/squares/tree/f2e24e07be8c94fa3ac603c3534dce7c454da99b)
  and
  [license](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/LICENSE)
- [Claude reader](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/claude.py),
  [shared model](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/model.py),
  [reader registry](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/reader.py)
  and
  [shell resolver](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/logrollup/shell.py)
- [Codex task-tree scanner](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py)
  and
  [delta receipt](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_task_tree_delta.py)
- [Session closer](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/close_session.py),
  [rollup checker](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/check_session_rollups.py)
  and
  [PR cost renderer](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/render_pr_rollup.py)
- [Codex rollup tests](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_log_rollup.py),
  [delta tests](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_task_tree_delta.py)
  and
  [Claude reader tests](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_log_rollup.py)
- [Resource usage README](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/resource-usage/README.md),
  [agent sessions README](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/campaign/agent-sessions/README.md)
  and
  [operating rules](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/operating-rules.md)
- [PR 156 usage delta audit](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/docs/project/reviews/review-2026-09-13-pr156-usage-delta.md)
  and
  [research throughput review](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/docs/project/reviews/review-2026-09-07-research-throughput-and-timeboxes.md)

Codex at commit `6b9826e` (`rust-v0.154.0`), for the format facts:

- [Repository](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340)

Related project documents:

- [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md)
- [urollup design](../../urollup-design.md)
- [Portable research brief](research-2026-09-13-portable-agent-usage.md)
- [metaproc and qm review](research-2026-09-14-metaproc-code-review.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
