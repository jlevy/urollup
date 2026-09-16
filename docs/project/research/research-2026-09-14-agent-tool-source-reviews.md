---
title: Agent Tool Source Reviews
description: Detailed evidence from read-only source reviews of OpenAI Codex, ccusage, Pi, agentfdr and Anthropic's session-report and receipts plugins at pinned commits, with reuse tables, dialect facts, pitfalls, unverified items and where each recommendation landed in the urollup design, plan and beads.
date: 2026-09-14
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete for beads uro-r9ep, uro-57bk and uro-tz8l; condensed into the portable research brief and applied in the urollup design, plan and implementation beads, with the remaining partial and open recommendations listed under Recommendations Status
---
# Research: Agent Tool Source Reviews

## Overview

urollup reads Claude Code, Codex and Pi logs and must count each request once, so its
design rests on what the agents write and on how existing usage tools read those
records. On 2026-09-14, five projects were reviewed read-only at pinned commits: the
OpenAI Codex source, ccusage, Pi, agentfdr, and Anthropic’s session-report plugin with
its sibling receipts plugin.

This brief keeps the full evidence from those reviews: reuse tables, dialect facts with
pinned source links, pitfalls, heuristics and still-unverified items.
Two documents build on it:

- The [portable research brief](research-2026-09-13-portable-agent-usage.md) condenses
  these findings in its
  [Existing Implementations](research-2026-09-13-portable-agent-usage.md#existing-implementations),
  [Log Dialects and Session Linkage](research-2026-09-13-portable-agent-usage.md#log-dialects-and-session-linkage),
  [Reusable Code and Tests](research-2026-09-13-portable-agent-usage.md#reusable-code-and-tests)
  and References sections.
- The [urollup design](../../urollup-design.md) turns them into rules, mainly in its
  discovery, snapshot, reconciliation, measure, price table and current-session
  sections.

[Recommendations Status](#recommendations-status) maps every review recommendation to
the design section, brief section or bead that applied it, and marks the rest partial,
queued or open.

The results that shaped the design most:

- **Codex:** `codex exec --json` usage is the thread’s cumulative total, not the turn’s.
  From `rust-v0.153.0`, `token_usage_record` gives one keyed record per response, while
  older rollouts need counter epochs and child-boundary rules because `token_count`
  events include synthetic totals and copied history.
- **ccusage:** its reports disagree by command path, it silently drops some records, and
  its pricing is fuzzy, floating-point and refreshed over the network, so it is a
  behavioral baseline with portable code and tests rather than an accounting authority.
- **Pi:** fork, clone and export copies keep entry IDs, which are only 8 hex characters
  unique within one file; usage also lives on tool results and summaries, and
  `pi-events` repeats one message’s usage in five event types.
- **agentfdr:** it counts one file at a time and double-counts repeated Codex snapshots,
  but its anomaly detectors and subagent spawn placement are reusable with tests.
- **session-report and receipts:** their dedupe depends on processing order, receipts
  observed that about 13% of one response’s block records disagree on `output_tokens`,
  and both follow a deterministic-engine, thin-skill reporting model.

## Questions to Answer

1. What do Codex rollouts, `codex exec` streams, Pi session files and Pi event streams
   record about usage, identity, forks and subagents, according to their source?
2. How do ccusage, agentfdr, session-report and receipts read Claude Code and Codex
   logs, and where do they miscount?
3. Which code, tests and fixtures can urollup port, under which license terms and
   privacy limits?
4. Which heuristics are worth offering as labeled estimates, and which should urollup
   avoid?
5. Where did each review recommendation land in the portable brief, the design, the plan
   and the beads?

## Scope

| Source | Revision | License | Scope read | Bead |
| --- | --- | --- | --- | --- |
| [openai/codex](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340) | `6b9826e` (tag `rust-v0.154.0`) | Apache-2.0, with `NOTICE` | `codex-rs/` protocol, rollout recorder, file names, compression, archiving and migration, history, exec events and JSON output, session and subagent spawning, shell environment, hooks, SQLite state, and their tests | `uro-r9ep` |
| [ccusage](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1) | `bd7f89b` (tag `v20.0.20`, 2026-08-15), plus later `main` commits up to `95bbc41` (2026-09-14) | MIT | The Rust Claude, Codex, Pi and common adapters, core, binary, unified loader, terminal and test-support crates, and the benchmark generator; other adapters through their READMEs and dedupe code | `uro-57bk` |
| [earendil-works/pi](https://github.com/earendil-works/pi/tree/d981de1229ef899957bbe968bc8dcda02a21f477) | `d981de1` (coding-agent 0.85.1, released 2026-09-05) | MIT, “Copyright (c) 2025 Mario Zechner” | `packages/coding-agent` session manager, runtime, JSON mode, bash tool, usage totals, cache stats, docs, changelog, tests and fixtures; `packages/ai` message types, cost and provider usage mapping; `packages/agent` loop and the experimental v4 harness session store | `uro-tz8l` |
| [kamihork/agentfdr](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90) | `e0904bf` (0.8.0) | MIT, “Copyright (c) 2026 kamihork” | All of `src/` except the i18n and HTML UI bodies; all of `test/` | `uro-tz8l` |
| [anthropics/claude-plugins-official](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537) | `f0dce59` | Apache-2.0 (plugin-level `LICENSE`, no `NOTICE` file) | `plugins/session-report` in full; `plugins/receipts` as an adjacent Anthropic usage miner with stronger dedupe notes and skill rules | `uro-tz8l` |

Excluded:

- Runtime behavior: conclusions come from source, tests, docs and changelogs, and only
  one synthetic probe ran any code (see [Methodology](#methodology))
- Local agent logs, which none of these reviews read
- ccusage’s other agent adapters beyond an index of their storage locations, because
  urollup’s scope excludes those agents
- squares and metaproc, which have their own reviews
  ([squares](research-2026-09-14-squares-code-review.md),
  [metaproc and qm](research-2026-09-14-metaproc-code-review.md))

Conventions used below:

- **Links:** every source link is pinned to the revision above.
  Codex link text drops the `codex-rs/` prefix, ccusage link text starts with
  `ccusage/<path>:<lines>` and then cites the same file by a short name or a bare line
  range, and Pi link text is relative to `packages/` and then shortened.
- **Reuse modes:** **port code** copies or closely translates source with its license
  notice, source path and commit; **port logic** reimplements a rule from its
  description with a citation; **adapt tests or fixtures** rewrites cases as synthetic
  urollup fixtures with urollup’s expected results; a **format fact** is dialect or
  pricing behavior to record; **learn only** takes a lesson without code; **avoid**
  names behavior urollup must not copy; a **watch item** is a format to track.
- **Inferred** marks a conclusion drawn from reading code that no test or doc in the
  source states.
- **References:** “design §N” names a section of the
  [urollup design](../../urollup-design.md), “brief” the
  [portable research brief](research-2026-09-13-portable-agent-usage.md), and `uro-…` a
  tbd bead.
- *(2026-09-15: the reviews cited sections of the plan spec and of a separate
  architecture and data contracts document, which the design doc later absorbed in bead
  `uro-i2rq`; those citations now name design sections.)*

## Licenses and Fixture Privacy

Ported code keeps its license notice and attribution, recorded with the source path and
commit in a third-party notices file, as
[design Decision 3](../../urollup-design.md#decision-3-code-reuse-and-licensing)
requires and bead `uro-phi8` implements.

- **ccusage (MIT):** “Copyright (c) 2025 ryoppippi”
  ([`ccusage/LICENSE:1-21`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/LICENSE#L1-L21)).
  Ported code and ported tests keep that notice in the file header or a third-party
  notices file, with source path and commit.
  Its embedded price snapshots are LiteLLM and models.dev data, not ccusage’s own work.
- **Pi and agentfdr (MIT):** ported code keeps the copyright line and license text (Pi:
  “Copyright (c) 2025 Mario Zechner”; agentfdr: “Copyright (c) 2026 kamihork”).
- **Codex (Apache-2.0):** ported code keeps the license text, a statement of changes and
  the `NOTICE` attribution, “OpenAI Codex / Copyright 2025 OpenAI”.
- **session-report and receipts (Apache-2.0, no `NOTICE`):** ported code keeps the
  license text, a statement of changes and attribution; logic reimplemented from
  descriptions, and the empirical facts cited here, need only citation.

Fixture privacy:

- ccusage’s Rust test fixtures are synthetic inline strings (`msg_123`, `session-a`,
  `project-a`, placeholder models).
  The exception is `ccusage/apps/ccusage/test/statusline-test*.json`, which hold the
  maintainer’s real home path, project path and a session UUID, so they must not be
  copied.
- Pi’s checked-in `before-compaction.jsonl` and `large-session.jsonl` fixtures contain
  real prompts and paths from the maintainer’s sessions; MIT permits reuse, but
  urollup’s public-fixture rule allows only structure-only derivatives.
- agentfdr’s in-test fixtures are fully synthetic.
- Codex checks in no rollout JSONL files; its tests build records inline.

## Codex Findings

Codex was reviewed at commit `6b9826e` (tag `rust-v0.154.0`), and every link below is
pinned to it.
Version bounds come from source snapshots at release tags and are stated as
“absent at one release, present at the next”, not from commit bisection.
Codex documents no stable rollout format, so these facts describe `rust-v0.154.0`
source, not observed runtime behavior.

### Questions Resolved From Source

Questions raised by earlier reviews and the portable brief, answered from Codex source:

| Question (raised by) | Answer from source | Confidence |
| --- | --- | --- |
| Is `codex exec --json` `turn.completed.usage` per turn or cumulative? (metaproc review, portable brief) | **Cumulative thread total**, not per turn. `usage_from_last_total` copies `ThreadTokenUsage.total`, never `.last` [`exec/src/event_processor_with_jsonl_output.rs:117-128`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L117-L128), [`exec/src/event_processor_with_jsonl_output.rs:502-528`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L502-L528). After `codex exec resume`, the total is seeded from the rollout, so it includes earlier runs [`core/src/session/mod.rs:1464-1471`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1464-L1471). This corrects the portable brief’s earlier per-turn description, fixed there on 2026-09-14. | High (code); the only exec usage test sets total equal to last [`exec/tests/event_processor_with_json_output.rs:1236-1303`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/tests/event_processor_with_json_output.rs#L1236-L1303) |
| First Codex release with `token_usage_record` (portable brief) | Absent at `rust-v0.152.0` and `0.152.1`; present in `protocol.rs` and the history wire enum at `rust-v0.153.0`. | High (tag snapshots) |
| Does a Codex subagent’s tool process get its own thread ID or its parent’s? (portable brief) | **Its own.** Shell tools insert `CODEX_THREAD_ID = context.session.thread_id` of the child session [`core/src/unified_exec/process_manager.rs:1370-1377`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/unified_exec/process_manager.rs#L1370-L1377); `CODEX_SESSION_ID` is the shared root session [`core/src/exec_env.rs:40-50`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs#L40-L50), [`core/src/session/session.rs:776-797`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/session.rs#L776-L797). | High (code); not observed in a running process |
| Which releases write `token_count` with `info: null`? (squares review) | `TokenCountEvent.info` is `Option` in every tag checked from `rust-v0.100.0` on. At this commit `info` is null whenever the session has no usage yet: a `token_count` is emitted for rate limits alone, for example on a first-turn usage-limit error [`core/src/session/turn.rs:1489-1494`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L1489-L1494) or after a response whose headers carried rate limits but no usage was recorded [`core/src/session/turn.rs:2631-2636`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2631-L2636). Resume and fork seed `info` from the last non-null value [`core/src/session/mod.rs:1656-1661`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1656-L1661), so null appears only before a thread’s first recorded usage. | High for 0.154 semantics; the first release is not bounded |
| Do legacy subagent replays keep their parent turn IDs? (squares review) | At this commit, yes. A legacy full-history subagent fork copies parent `event_msg` records verbatim, including `task_started`/`task_complete` with the parent’s `turn_id`, and keeps `turn_context` (with `turn_id`) for full-history forks [`core/src/agent/control/spawn.rs:65-105`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105), [`core/src/agent/control/spawn.rs:1007-1066`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L1007-L1066). Copied lines get a new write-time `timestamp` [`rollout/src/recorder.rs:1977-1994`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1977-L1994), which explains squares’ near-zero replay intervals. Older releases are unverified. | High (0.154) |
| Where do subagent-owned records start in legacy child rollouts that lack `subagent_history_start_ordinal`? (squares review) | Since `rust-v0.152.0`, `thread_settings_applied` carries `thread_id`, and “copied snapshots retain their original owner’s ID” [`protocol/src/protocol.rs:2183-2191`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2183-L2191). The child appends its own settings event after the copied prefix in one batch [`core/src/session/mod.rs:1495-1519`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1495-L1519), [`core/src/session/thread_settings.rs:119-124`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/thread_settings.rs#L119-L124). The first `thread_settings_applied` whose `thread_id` equals the file’s thread is a native boundary; squares’ heuristics remain the fallback for pre-0.152 files. | High (0.152+) |
| Does Codex record the git branch? (the squares review said no) | Yes: `session_meta.payload.git` has `commit_hash`, `branch`, `repository_url` [`protocol/src/protocol.rs:3334-3348`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3334-L3348), mirrored as `threads.git_branch` [`state/migrations/0001_threads.sql:1-19`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/migrations/0001_threads.sql#L1-L19). It is per thread, not per request. | High |
| Is `guardian_review` a spawn descendant? (squares review) | A guardian trunk review is a `SubAgent(Other("guardian"))` session with `parent_thread_id` and `thread_source: guardian_review` [`core/src/codex_delegate.rs:82-116`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/codex_delegate.rs#L82-L116), [`core/src/guardian/review.rs:236-242`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/guardian/review.rs#L236-L242), so it is a parent-linked child. Parallel reviews run as **ephemeral forks with no rollout** [`core/src/guardian/review_session.rs:787-797`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/guardian/review_session.rs#L787-L797), so their usage never reaches disk. | High |
| Codex signals in cloud environments (portable brief) | Not covered by this source review. The thread store is only `Local` or `InMemory` [`core/src/config/mod.rs:599-605`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/config/mod.rs#L599-L605); hook `transcript_path` is null without a local thread. | Still unverified |

### Rollout Files, Layout and Rotation

- **Roots:** `$CODEX_HOME/sessions` and `$CODEX_HOME/archived_sessions`
  [`rollout/src/lib.rs:83-84`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/lib.rs#L83-L84).
- **Active path:** `sessions/YYYY/MM/DD/rollout-YYYY-MM-DDTHH-MM-SS-<thread_id>.jsonl`,
  where the date directories and the name’s timestamp use **local time**
  (`OffsetDateTime::now_local`)
  [`rollout/src/recorder.rs:1635-1657`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1635-L1657).
  Codex’s own parser then reads that timestamp with `assume_utc`
  [`rollout/src/rollout_file_name.rs:39-60`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/rollout_file_name.rs#L39-L60),
  so file-name times are local wall-clock values, not UTC.
  `session_meta.payload.timestamp` and every line’s `timestamp` are UTC with
  milliseconds
  [`rollout/src/recorder.rs:866-872`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L866-L872),
  [`rollout/src/recorder.rs:1977-1994`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1977-L1994).
- **Revert suffix:** a reverted paginated thread gets a new immutable file
  `rollout-<ts>-<thread_id>_<rollout_id>.jsonl` that references the retained prefix; old
  files stay intact
  [`rollout/src/rollout_file_name.rs:62-76`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/rollout_file_name.rs#L62-L76),
  [`rollout/src/recorder.rs:93-104`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L93-L104),
  [`thread-store/src/local/revert_thread.rs:15-18`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/revert_thread.rs#L15-L18).
- **No rotation:** a thread never rotates by size or date.
  Resume reopens the same file for append (decompressing a `.zst` first)
  [`rollout/src/recorder.rs:917-930`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L917-L930),
  [`rollout/src/recorder.rs:1924-1935`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1924-L1935).
  A long thread stays in its creation-date directory.
- **Deferred creation:** a new thread’s file is created only on first persist
  [`rollout/src/recorder.rs:829-834`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L829-L834),
  [`rollout/src/recorder.rs:906-915`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L906-L915);
  `--ephemeral` or `config.ephemeral` writes no rollout at all
  [`core/src/session/session.rs:855-858`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/session.rs#L855-L858),
  [`exec/src/cli.rs:35-37`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/cli.rs#L35-L37).
- **One thread, several files:** (a) revert chains (`_<rollout_id>`), (b) paginated
  forks, whose `history_base` points into another rollout by rollout ID, end ordinal and
  byte offset
  [`protocol/src/protocol.rs:3018-3032`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3018-L3032),
  [`thread-store/src/local/rollout_lineage.rs:14-26`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_lineage.rs#L14-L26).
  Legacy threads are one file each.
- **Compression (`.jsonl.zst`):** a background worker compresses files whose mtime is at
  least 7 days old, in both roots, at zstd level 3, and deletes the plain file
  [`rollout/src/compression.rs:253-260`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L253-L260),
  [`rollout/src/compression.rs:610-679`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L610-L679),
  [`rollout/src/compression.rs:687-711`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L687-L711).
  It is gated by feature `local_thread_store_compression`, stage under development,
  default off
  [`features/src/lib.rs:1112-1117`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/features/src/lib.rs#L1112-L1117),
  [`core/src/thread_manager.rs:383-413`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/thread_manager.rs#L383-L413).
  A plain file hides its compressed sibling during discovery
  [`rollout/src/compression.rs:156-172`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L156-L172).
  Resume decompresses back to plain and removes the `.zst`
  [`rollout/src/compression.rs:73-123`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L73-L123),
  so one rollout can alternate representations.
- **Archive:** archiving renames every rollout of the thread (and of spawned
  descendants) into **flat** `archived_sessions/<file name>`; unarchive moves it back to
  the date directory parsed from the name
  [`thread-store/src/local/archive_thread.rs:55-63`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/archive_thread.rs#L55-L63),
  [`thread-store/src/local/archive_thread.rs:80-119`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/archive_thread.rs#L80-L119),
  [`thread-store/src/local/unarchive_thread.rs:57-91`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/unarchive_thread.rs#L57-L91).
  Tests also read date-nested archived layouts
  [`core/tests/suite/rollout_list_find.rs:229-244`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/rollout_list_find.rs#L229-L244).
- **In-place migration:** `codex migrate-rollouts --apply`
  [`cli/src/main.rs:215-216`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/cli/src/main.rs#L215-L216),
  and a startup migration behind feature `background_paginated_rollout_migration`
  (default off)
  [`features/src/lib.rs:1124-1129`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/features/src/lib.rs#L1124-L1129),
  rewrite legacy rollouts as paginated at the same path
  [`thread-store/src/local/rollout_migration.rs:1-9`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration.rs#L1-L9).
  The canonicalizer “preserve[s] the model-visible conversation, not … every legacy
  record byte-for-byte”
  [`thread-store/src/local/rollout_migration/canonicalizer.rs:1-10`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L1-L10):
  it keeps `token_count` and `token_usage_record`
  [`thread-store/src/local/rollout_migration/canonicalizer.rs:240-301`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L240-L301),
  but rollback planning drops rolled-back records
  [`thread-store/src/local/rollout_migration/rollback_plan.rs:1-8`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/rollback_plan.rs#L1-L8)
  and legacy subagents are cut to a bounded suffix context
  [`thread-store/src/local/rollout_migration/subagent.rs:1-10`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/subagent.rs#L1-L10).
- **Line format:** `{timestamp, ordinal?, type, payload}` plus optional `metadata` on
  `response_item`
  [`history/src/lib.rs:253-264`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs#L253-L264),
  [`history/src/rollout_payload.rs:21-63`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/rollout_payload.rs#L21-L63).
  `ordinal` exists only in paginated history, with `session_meta` at 0
  [`rollout/src/ordinal.rs:16-53`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/ordinal.rs#L16-L53).
  Codex decodes lines through `serde_json::Value` because of an arbitrary-precision
  float bug
  [`rollout/src/lib.rs:39-73`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/lib.rs#L39-L73),
  takes the **first** `session_meta` as canonical and skips malformed lines
  [`rollout/src/recorder.rs:1026-1089`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1026-L1089).

### Rollout Item Types

Wire tags (`type`) from
[`history/src/rollout_payload.rs:21-63`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/rollout_payload.rs#L21-L63):
`session_meta`, `response_item`, `inter_agent_communication`,
`inter_agent_communication_metadata`, `compacted`, `turn_context`, `token_usage_record`,
`world_state`, `retained_context`, `security_risk_score`, `event_msg`, `realtime_item`.

- **`session_meta`**
  [`protocol/src/protocol.rs:3034-3102`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3034-L3102):
  `session_id` (root thread, since `rust-v0.142.0`; filled from `id` when absent
  [`protocol/src/protocol.rs:3144-3171`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3144-L3171)),
  `id`, `forked_from_id`, `forked_from_ordinal_exclusive` (0.152+), `parent_thread_id`
  (top-level by 0.140), `timestamp`, `cwd`, `originator` (`codex_exec` for exec
  [`exec/src/lib.rs:260`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/lib.rs#L260),
  default `codex_cli_rs`
  [`login/src/auth/default_client.rs:40`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/login/src/auth/default_client.rs#L40)),
  `cli_version` (the writer’s crate version
  [`rollout/src/recorder.rs:874-884`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L874-L884)),
  `source`, `thread_source`, `agent_nickname`, `agent_role` (alias `agent_type`),
  `agent_path`, `model_provider`, `base_instructions`, `dynamic_tools`,
  `selected_capability_roots`, `memory_mode`, `history_mode` (`legacy` default or
  `paginated`, 0.145+
  [`protocol/src/protocol.rs:773-789`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L773-L789)),
  `history_base`, `subagent_history_start_ordinal` (0.145+), `multi_agent_version`,
  `context_window`; plus sibling `git`
  [`protocol/src/protocol.rs:3137-3142`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3137-L3142).
  There is no `instructions` field any more
  [`protocol/src/protocol.rs:3034-3038`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3034-L3038).
- **`source`**
  [`protocol/src/protocol.rs:2740-2754`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2740-L2754):
  `cli`, `vscode`, `exec`, `mcp`, `{custom}`,
  `{internal: memory_consolidation|guardian}`,
  `{subagent: review|compact|memory_consolidation|{thread_spawn:{parent_thread_id, depth, agent_path, agent_nickname, agent_role}}|{other}}`
  [`protocol/src/protocol.rs:2815-2840`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2815-L2840);
  unknown values deserialize as `unknown`. `thread_source` is `user`, `subagent`,
  `guardian_review` (0.150+), `memory_consolidation` or any other string as a feature
  name
  [`protocol/src/protocol.rs:2757-2812`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2757-L2812).
- **`turn_context`**
  [`protocol/src/protocol.rs:3197-3255`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3197-L3255):
  `turn_id`, `root_turn_id` (subagent turns only), `cwd`, `current_date`, `timezone`,
  approval and sandbox fields, `model`, `effort`, `collaboration_mode`,
  `multi_agent_version`, `summary` (compat-only).
  Written once per real user turn and after mid-turn compaction.
- **`token_usage_record`**
  [`protocol/src/protocol.rs:2237-2248`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2237-L2248):
  see [Codex Token Usage](#codex-token-usage).
- **`compacted`**
  [`history/src/lib.rs:185-203`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs#L185-L203),
  [`history/src/rollout_payload.rs:154-180`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/rollout_payload.rs#L154-L180):
  `message`, `replacement_history`, window IDs, `compaction_response_id`, and
  **`latest_token_usage_record`, a copy** of the last record kept for resume.
- **`event_msg`** persisted payloads
  [`rollout/src/policy.rs:92-206`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/policy.rs#L92-L206):
  always `token_count`, `thread_goal_updated`, `thread_rolled_back`, `turn_aborted`,
  `task_started` (`turn_id`, `trace_id`, `started_at`, `model_context_window`,
  `collaboration_mode_kind`), `task_complete` (`turn_id`, `error`, `started_at`,
  `completed_at`, `duration_ms`, `time_to_first_token_ms`) and `thread_settings_applied`
  [`protocol/src/protocol.rs:2140-2191`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2140-L2191),
  [`protocol/src/protocol.rs:1403-1419`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L1403-L1419);
  `item_completed` in paginated history (legacy keeps only some); legacy-only
  `user_message`, `agent_message`, `agent_reasoning`, `context_compacted`,
  `mcp_tool_call_end`, `web_search_end`, review-mode and patch events.
  `error`, `warning`, `model_reroute`, `raw_response_completed` and `exec_command_*` are
  never persisted.
- **`response_item`**: Responses API items (`message`, `reasoning`, `function_call`,
  `function_call_output`, `custom_tool_call*`, `local_shell_call`, `web_search_call`,
  `image_generation_call`, `compaction`, `context_compaction` …)
  [`rollout/src/policy.rs:42-65`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/policy.rs#L42-L65).
- **Retired records** Codex skips: `ghost_snapshot` response items and
  `guardian_assessment`, `thread_name_updated`, `undo_completed` events; legacy
  `rate_limits.*.resets_at` was once an RFC 3339 string
  [`thread-store/src/local/rollout_migration/line_parser.rs:47-58`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/line_parser.rs#L47-L58),
  [`thread-store/src/local/rollout_migration/line_parser.rs:110-135`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/line_parser.rs#L110-L135).

### Codex Token Usage

- **`TokenUsage`**
  [`protocol/src/protocol.rs:2215-2235`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2215-L2235):
  `input_tokens`, `cached_input_tokens`, `cache_write_input_tokens` (`serde(default)`;
  absent at 0.144.0, present at 0.145.0), `output_tokens`, `reasoning_output_tokens`,
  `total_tokens`. `codex_rollout_budget_units` is never serialized.
- **Inclusion:** values are copied verbatim from the Responses API `usage`
  (`input_tokens_details.cached_tokens`, `cache_write_tokens`,
  `output_tokens_details.reasoning_tokens`, provider `total_tokens`)
  [`codex-api/src/sse/responses.rs:138-166`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/codex-api/src/sse/responses.rs#L138-L166).
  Codex treats cached as a subset of input (`non_cached_input = input - cached`) and
  adds no reasoning to output in its blended total
  [`protocol/src/protocol.rs:2396-2412`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2396-L2412),
  which confirms the portable brief.
  Whether `cache_write_input_tokens` is inside `input_tokens` is not stated in source
  (unverified).
- **`token_count` event**
  [`protocol/src/protocol.rs:2317-2321`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2317-L2321):
  `{info: {total_token_usage, last_token_usage, model_context_window} | null, rate_limits | null}`.
  `total_token_usage += last` on each observed response
  [`protocol/src/protocol.rs:2259-2289`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2259-L2289),
  [`core/src/context_manager/history.rs:624-634`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/context_manager/history.rs#L624-L634).
  Emission sites:
  - after each sampling request’s tools drain, once per response batch
    [`core/src/session/turn.rs:2644-2689`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2644-L2689),
    [`core/src/session/turn.rs:2857-2863`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2857-L2863);
  - **rate-limit only** updates with unchanged `info`
    [`core/src/session/mod.rs:4487-4501`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4487-L4501);
  - **estimates after compaction:**
    `last_token_usage = {total_tokens: estimate, all else 0}`, total unchanged
    [`core/src/session/mod.rs:4447-4485`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4447-L4485)
    (called from
    [`core/src/compact.rs:400`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/compact.rs#L400),
    [`core/src/compact_remote.rs:308`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/compact_remote.rs#L308),
    [`core/src/session/handlers.rs:346`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/handlers.rs#L346));
  - **context window exceeded:** `total = {total_tokens: window, all else 0}` and
    `last = {total_tokens: window - previous_total, all else 0}`
    [`core/src/session/turn.rs:1484-1488`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L1484-L1488),
    [`protocol/src/protocol.rs:2291-2314`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2291-L2314).
    Later appends add to these zeroed components, so cumulative input and output fields
    restart while `total_tokens` jumps.
    This explains the decreasing cumulative total that the portable brief saw in local
    logs.
  - on resume and fork, `info` is seeded from the last non-null `token_count` in the
    loaded history
    [`core/src/session/mod.rs:1464-1471`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1464-L1471),
    [`core/src/session/mod.rs:1486-1493`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1486-L1493).
- **`rate_limits`**
  [`protocol/src/protocol.rs:2323-2391`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2323-L2391):
  `limit_id`, `limit_name`, `normal_model_slug`, `primary` and `secondary` windows
  (`used_percent` 0–100 float, `window_minutes`, `resets_at` Unix seconds),
  `credits {has_credits, unlimited, balance}`,
  `individual_limit {limit, used, remaining_percent, resets_at}`,
  `spend_control_reached`, `plan_type` (lowercase enum, unknown values become `unknown`
  [`protocol/src/account.rs:9-44`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/account.rs#L9-L44)),
  `rate_limit_reached_type`. Parsed from `x-codex-*` headers, one snapshot per metered
  `limit_id`
  [`codex-api/src/rate_limits.rs:23-100`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/codex-api/src/rate_limits.rs#L23-L100).
  The session keeps **one** latest snapshot and carries forward missing `credits`,
  `individual_limit`, `spend_control_reached` and `plan_type` from the previous one,
  defaulting `limit_id` to `codex`
  [`core/src/state/session.rs:278-289`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L278-L289),
  [`core/src/state/session.rs:388-411`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L388-L411).
  So each `token_count` repeats the latest snapshot, carried-forward fields may be
  stale, and when one response reports several `limit_id` buckets only the last reaches
  the rollout
  [`codex-api/src/sse/responses.rs:77-79`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/codex-api/src/sse/responses.rs#L77-L79).
- **`token_usage_record`** (0.153.0+)
  [`protocol/src/protocol.rs:2237-2248`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2237-L2248):
  `thread_id`, `turn_id`, `session_id` (root), `root_turn_id` (the root turn that caused
  a subagent turn), `response_id`, `usage` (this response), `turn_token_usage` and
  `thread_token_usage` (running sums).
  Written once per completed response **that reports usage**; a response without usage
  writes nothing
  [`core/src/session/mod.rs:4377-4409`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4377-L4409),
  [`core/src/state/session.rs:161-197`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L161-L197),
  [`core/tests/suite/token_usage_rollout.rs:31-116`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/token_usage_rollout.rs#L31-L116).
  Local and v2 remote compaction requests record usage too
  [`core/src/compact.rs:796-816`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/compact.rs#L796-L816),
  [`core/src/compact_remote_v2.rs:449`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/compact_remote_v2.rs#L449);
  legacy `/responses/compact` records none
  [`core/src/compact_remote.rs:289-308`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/compact_remote.rs#L289-L308).
- **Model identity:** `turn_context.model` is the requested model.
  A server reroute raises only an unpersisted warning
  [`core/src/session/turn.rs:2589-2601`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2589-L2601),
  [`rollout/src/policy.rs:163`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/policy.rs#L163),
  so the served model is not in rollouts.
- **SQLite `tokens_used`** is the last
  `token_count.info.total_token_usage.total_tokens`, ignoring records
  [`state/src/extract.rs:109-115`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/extract.rs#L109-L115);
  it is not a usage ledger.

### Streamed `codex exec --json` Output

- **Events**
  [`exec/src/exec_events.rs:8-57`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs#L8-L57):
  `thread.started {thread_id}`, `turn.started {}`, `turn.completed {usage}`,
  `turn.failed {error:{message}}`, `item.started|updated|completed {item}`,
  `error {message}`. `--experimental-json` is an alias
  [`exec/src/cli.rs:58-65`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/cli.rs#L58-L65).
  No schema or version field.
- **`usage`**
  [`exec/src/exec_events.rs:59-73`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs#L59-L73):
  `input_tokens`, `cached_input_tokens`, `cache_write_input_tokens` (default 0),
  `output_tokens`, `reasoning_output_tokens`; **no `total_tokens`**. The doc comments
  say “during the turn”, but the value is the thread’s cumulative total (see
  [Questions Resolved From Source](#questions-resolved-from-source)). It covers only the
  primary thread: exec filters notifications to its own `thread_id` and `turn_id`
  [`exec/src/lib.rs:1577-1604`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/lib.rs#L1577-L1604),
  so **subagent usage is excluded**. If no usage notification arrived, every field is 0
  [`exec/src/event_processor_with_jsonl_output.rs:117-120`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L117-L120).
  Usage notifications are not guaranteed delivery on the in-process channel, unlike
  `TurnCompleted`
  [`app-server/src/in_process.rs:111-126`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/app-server/src/in_process.rs#L111-L126),
  so the value can be stale (read from code, untested).
- **One turn per stream:** exec shuts down after the first terminal turn status
  [`exec/src/event_processor_with_jsonl_output.rs:518-558`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L518-L558).
- **Failed turns:** `turn.failed` carries no usage; **interrupted turns emit nothing**
  [`exec/src/event_processor_with_jsonl_output.rs:531-556`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L531-L556).
  `error` events are also emitted for retried errors.
  Usage from responses completed before a failure is still in the rollout’s
  `token_count` and `token_usage_record` lines.
- **Missing from the stream:** timestamps, model, turn ID, response ID, rollout path;
  item IDs are a local counter `item_N`
  [`exec/src/event_processor_with_jsonl_output.rs:99-101`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L99-L101).
- **Shared keys with the rollout:** `thread.started.thread_id` equals
  `session_meta.payload.id` and the file name’s thread ID
  [`exec/src/event_processor_with_jsonl_output.rs:396-400`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L396-L400),
  [`rollout/src/recorder.rs:874-877`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L874-L877);
  the rollout has `source: exec` and `originator: codex_exec`
  [`exec/src/lib.rs:708-716`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/lib.rs#L708-L716).
  Non-ephemeral exec threads are `paginated` since `rust-v0.148.0`
  [`exec/src/lib.rs:1365-1366`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/lib.rs#L1365-L1366).

### Subagents, Forks and Resume

- **Spawn:** `thread_spawn` children get `parent_thread_id`, `forked_from_id = parent`
  when forked from context, `thread_source: subagent`, and a new thread ID
  [`core/src/agent/control/spawn.rs:1108-1123`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L1108-L1123);
  `session_id` is the root’s
  [`core/src/session/session.rs:776-797`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/session.rs#L776-L797).
  Review subagents use `SubAgent(Review)` through the delegate path with
  `parent_thread_id`
  [`core/src/tasks/review.rs:128-136`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/tasks/review.rs#L128-L136),
  [`core/src/codex_delegate.rs:106-116`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/codex_delegate.rs#L106-L116).
- **What a subagent fork copies**
  [`core/src/agent/control/spawn.rs:65-105`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105),
  [`core/src/agent/control/spawn.rs:1007-1066`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L1007-L1066):
  user, developer and system messages, final answers, `compacted` (with
  `latest_token_usage_record` cleared), `session_meta` (the parent’s, a foreign line),
  all `event_msg`, and `turn_context` for full-history forks.
  **`token_usage_record` is always dropped** (since 0.153.0). In a **paginated**
  destination `token_count`, `item_completed`, `thread_goal_updated` and
  `thread_settings_applied` are also dropped; in a **legacy** destination the parent’s
  `token_count` events are copied.
- **Paginated subagents** persist the inherited prefix at ordinals `1..start-1` and set
  `subagent_history_start_ordinal = prefix_len + 1`
  [`thread-store/src/live_thread.rs:115-145`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/live_thread.rs#L115-L145);
  Codex refuses to resume a child whose prefix is incomplete
  [`rollout/src/ordinal.rs:86-96`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/ordinal.rs#L86-L96).
- **User forks:** legacy forks copy the parent rollout (including its `token_count`,
  `token_usage_record` and `compacted.latest_token_usage_record` lines) into the child
  file (`ForkPersistence::Copied`)
  [`core/src/thread_manager.rs:1332-1360`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/thread_manager.rs#L1332-L1360),
  [`core/src/session/mod.rs:1513-1519`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1513-L1519);
  paginated forks reference the parent through `history_base` and copy nothing
  (`Referenced`)
  [`core/src/thread_manager.rs:1362-1397`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/thread_manager.rs#L1362-L1397),
  [`core/src/session/mod.rs:1497-1506`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1497-L1506).
  Both seed the child’s cumulative `token_count` and (for copied forks) its
  `thread_token_usage` from the parent’s last values
  [`core/src/session/mod.rs:1486-1493`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1486-L1493),
  [`core/src/state/session.rs:178-184`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L178-L184);
  the app-server replays the parent’s totals to the forked thread
  [`app-server/tests/suite/v2/thread_fork.rs:1157-1208`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/app-server/tests/suite/v2/thread_fork.rs#L1157-L1208).
- **`forked_from_ordinal_exclusive`** is explicit from 0.152.0; older files infer it
  from `history_base` only when unambiguous
  [`rollout/src/metadata.rs:117-135`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/metadata.rs#L117-L135).
- **Resume** keeps the thread ID and file, appends, and seeds counters from the file
  [`core/src/session/mod.rs:1464-1471`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1464-L1471).
  `turn_token_usage` restarts per turn and `thread_token_usage` continues
  [`core/tests/suite/token_usage_rollout.rs:89-107`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/token_usage_rollout.rs#L89-L107).
- **Legacy rollouts:** before 0.142 `session_meta` has no `session_id`; before 0.140 no
  top-level `parent_thread_id` (only inside `source.subagent.thread_spawn`); before
  0.145 no `history_mode` or subagent start ordinal; before 0.152 no settings
  `thread_id` or fork ordinal.
  Codex itself filters legacy subagent `session_id` values synthesized from the child’s
  own ID
  [`core/src/session/session.rs:785-789`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/session.rs#L785-L789).

### Codex Session Identity

- **Tool environment:** `CODEX_THREAD_ID` (current thread; inserted after the shell
  environment policy, so config cannot remove it)
  [`protocol/src/shell_environment.rs:6-7`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/shell_environment.rs#L6-L7),
  [`protocol/src/shell_environment.rs:145-159`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/shell_environment.rs#L145-L159);
  `CODEX_SESSION_ID` (root) and `CODEX_VERSION`
  [`core/src/exec_env.rs:40-50`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs#L40-L50).
  Shell snapshots re-export live values
  [`core/src/tools/runtimes/mod.rs:264-285`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/tools/runtimes/mod.rs#L264-L285).
  No `CODEX_TURN_ID` or rollout-path variable exists.
- **Not set for:** MCP stdio servers (allowlisted environment)
  [`rmcp-client/src/utils.rs:16-59`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rmcp-client/src/utils.rs#L16-L59)
  and **hooks**, which replay a snapshot of the Codex process environment taken at
  session start
  [`hooks/src/registry.rs:73-82`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/registry.rs#L73-L82),
  [`hooks/src/engine/command_runner.rs:420-425`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/engine/command_runner.rs#L420-L425).
  A hook of a Codex started from another Codex tool shell sees the **outer** Codex’s
  variables.
- **Hook input**
  [`hooks/src/schema.rs:278-296`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs#L278-L296),
  [`hooks/src/schema.rs:604-622`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs#L604-L622):
  events `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`,
  `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `SubagentStart`, `SubagentStop`,
  `Stop`, `Interrupt`
  [`protocol/src/protocol.rs:1576-1589`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L1576-L1589).
  **`session_id` is always the root session ID**
  [`core/src/hook_runtime.rs:151-158`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L151-L158),
  [`core/src/hook_runtime.rs:432-444`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L432-L444).
  `transcript_path` is the current thread’s rollout (the child’s inside a subagent),
  null for ephemeral threads
  [`core/src/session/mod.rs:4638-4650`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4638-L4650);
  `SubagentStop` sets `transcript_path` to the **parent’s** rollout and
  `agent_transcript_path` to the child’s
  [`core/src/hook_runtime.rs:385-421`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L385-L421).
  `agent_id` is the child thread ID, sent only for `thread_spawn` subagents; other
  subagents run no start or stop hooks
  [`core/src/hook_runtime.rs:128-150`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L128-L150).
- **SQLite:** under `sqlite_home` (config), else `$CODEX_SQLITE_HOME`, else
  `$CODEX_HOME`
  [`state/src/lib.rs:108-109`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/lib.rs#L108-L109),
  [`core/src/config/mod.rs:3983-3988`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/config/mod.rs#L3983-L3988);
  fixed names `state_5.sqlite`, `logs_2.sqlite`, `goals_1.sqlite`, `memories_1.sqlite`,
  `queue_1.sqlite`, `thread_history_1.sqlite`
  [`state/src/sqlite.rs:29-34`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/sqlite.rs#L29-L34),
  WAL mode
  [`state/src/sqlite.rs:278-291`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/sqlite.rs#L278-L291).
  `threads` holds `id`, `rollout_path`, `source` (JSON with spawn parent), `cwd`,
  `model`, `reasoning_effort`, `tokens_used`, `archived`, git fields, `cli_version`,
  `agent_*`, `thread_source`, `history_mode`, `originator` and more (migrations
  0001–0054, first at
  [`state/migrations/0001_threads.sql:1-19`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/migrations/0001_threads.sql#L1-L19));
  `thread_spawn_edges(parent_thread_id,
  child_thread_id, status)`
  [`state/migrations/0021_thread_spawn_edges.sql:1-8`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/migrations/0021_thread_spawn_edges.sql#L1-L8);
  no `forked_from_id` column.
  It is a mirror backfilled from rollouts
  [`state/src/lib.rs:1-5`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/lib.rs#L1-L5),
  [`rollout/src/state_db.rs:130-192`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/state_db.rs#L130-L192),
  but for `paginated` threads SQLite is authoritative for metadata and for the current
  rollout path
  [`thread-store/src/local/thread_rollout_resolver.rs:91-94`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/thread_rollout_resolver.rs#L91-L94).
  Older binaries ignore newer migrations
  [`state/src/migrations.rs:13-28`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/migrations.rs#L13-L28).
  Usable read-only as a discovery hint, never a usage source.
- **`session_index.jsonl`** in `$CODEX_HOME` is an append-only thread-name index
  `{id, thread_name, updated_at}`
  [`rollout/src/session_index.rs:21-29`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/session_index.rs#L21-L29).

### Rollout Format Stability

- No document promises a stable rollout format; `docs/` only links to the developer
  site. Review guidance lists “resuming sessions from existing rollouts” as a
  breaking-change surface
  [`AGENTS.md:102-110`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/AGENTS.md#L102-L110).
  Compatibility is maintained through `serde(default)`, aliases
  (`task_started`/`turn_started`, `agent_type`/`agent_role`), legacy normalizers and
  per-variant wire-shape tests
  [`history/src/tests.rs:393-518`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/tests.rs#L393-L518).
- Codex **rejects** an unknown `history_mode` in a file’s own `session_meta`
  [`rollout/src/recorder.rs:1052-1057`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1052-L1057).
  The comment on `TurnContextItem.summary` says it “should be removed in a future schema
  cleanup”
  [`protocol/src/protocol.rs:3250-3254`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3250-L3254).
- Fields added between 0.140 and 0.154: `parent_thread_id`, `session_id`,
  `history_mode`, `history_base`, `subagent_history_start_ordinal`,
  `cache_write_input_tokens`, `guardian_review`, `forked_from_ordinal_exclusive`,
  settings `thread_id`, `token_usage_record`. Likely to change next: paginated migration
  and compression (both under development), `rate_limits` extensions
  (`individual_limit`, `spend_control_reached`), and `TODO` optional
  `model_context_window`
  [`protocol/src/protocol.rs:2250-2257`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2250-L2257).
- `RolloutLine` deliberately has no `Deserialize`; readers must decode through `Value`
  [`history/src/lib.rs:253-264`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs#L253-L264).

### Codex Tests and Fixtures

No rollout JSONL files are checked in; all samples are inline strings or Rust helpers.
The app-server schemas under `codex-rs/app-server-protocol/schema/` describe camelCase
app-server types, not rollout lines; no rollout schema is committed.

| Test | Covers |
| --- | --- |
| [`history/src/tests.rs:393-518`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/tests.rs#L393-L518) | One canonical JSON shape per rollout item type, including a full `token_usage_record` |
| [`rollout/src/tests.rs:52-74`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/tests.rs#L52-L74) | `token_count` with `info: null`, full `rate_limits`, `ordinal`; `metadata` kept only on `response_item` |
| [`core/tests/suite/token_usage_rollout.rs:31-116`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/token_usage_rollout.rs#L31-L116) | Per-response records across resume; response without usage writes none |
| [`core/src/session/tests.rs:2808-2936`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/tests.rs#L2808-L2936) | `token_count` with interleaved `info: None`; resume takes last non-null; record lookup stops at compaction |
| [`core/src/agent/control_tests.rs:1388-1523`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control_tests.rs#L1388-L1523) | Subagent fork does not inherit `token_usage_record` or `latest_token_usage_record` |
| [`core/tests/suite/client.rs:3213-3343`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/client.rs#L3213-L3343), [`core/tests/suite/client.rs:3441-3532`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/client.rs#L3441-L3532) | `token_count` built from `x-codex-*` headers; context-window-exceeded synthetic totals |
| [`core/tests/suite/compact.rs:990-1078`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/tests/suite/compact.rs#L990-L1078) | Compaction: zero API usage then an estimated `token_count`; `compacted.latest_token_usage_record` |
| [`app-server/tests/common/rollout.rs:96-153`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/app-server/tests/common/rollout.rs#L96-L153) | Helper that writes a rollout with an asymmetric total/last `token_count` |
| [`rollout/src/recorder_tests.rs:262-378`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder_tests.rs#L262-L378) | Legacy meta without `session_id`; `ghost_snapshot`; copied fork meta with an unknown `history_mode` |
| [`rollout/src/recorder_tests.rs:1001-1174`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder_tests.rs#L1001-L1174) | Paginated `token_count` with float `used_percent`; truncated tail; subagent start ordinal |
| [`rollout/src/compression_tests.rs:31-179`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression_tests.rs#L31-L179), [`rollout/src/compression_tests.rs:265-446`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression_tests.rs#L265-L446) | `.jsonl.zst` reading, junk before `session_meta`, invalid UTF-8 tail, archived fork chains |
| [`rollout/src/rollout_file_name_tests.rs:7-29`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/rollout_file_name_tests.rs#L7-L29) | Plain and revert file names |
| [`thread-store/src/local/rollout_migration/line_parser_tests.rs:17-228`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/line_parser_tests.rs#L17-L228) | Legacy `token_count` without `limit_id`, RFC 3339 `resets_at`, retired events |
| [`exec/tests/event_processor_with_json_output.rs:107-166`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/tests/event_processor_with_json_output.rs#L107-L166), [`exec/tests/event_processor_with_json_output.rs:1236-1303`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/tests/event_processor_with_json_output.rs#L1236-L1303), [`exec/tests/event_processor_with_json_output.rs:1540-1674`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/tests/event_processor_with_json_output.rs#L1540-L1674) | Exec `thread.started`, `turn.started`, usage (total equals last, so ambiguous), failed turns |
| [`exec/tests/suite/resume.rs:158-245`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/tests/suite/resume.rs#L158-L245), [`exec/tests/suite/resume.rs:884-1054`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/tests/suite/resume.rs#L884-L1054) | `exec resume` appends to the same paginated file; `exec fork` `thread.started` equals the child `payload.id` |
| [`sdk/typescript/tests/runStreamed.test.ts:14-64`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/sdk/typescript/tests/runStreamed.test.ts#L14-L64) | Exact exec event sequence with usage values |

### Codex Format Facts

Each fact names the bead or design section it affects and how it relates to earlier
research.

| Fact | Source | Effect on urollup | Relation to earlier research |
| --- | --- | --- | --- |
| `turn.completed.usage` is the cumulative thread total, without `total_tokens`, excluding subagents | [`exec/src/event_processor_with_jsonl_output.rs:117-128`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L117-L128), [`exec/src/exec_events.rs:59-73`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs#L59-L73), [`exec/src/lib.rs:1577-1604`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/lib.rs#L1577-L1604) | uro-y2qj `codex-exec` adapter; design §3.4; brief Usage Fields | Corrects |
| `token_usage_record` first shipped in `rust-v0.153.0` | tag snapshots; [`protocol/src/protocol.rs:2237-2248`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2237-L2248) | uro-y2qj version gate; uro-obx5 fixtures | Resolves |
| Record is per response with usage; no record for responses without usage; legacy remote compaction unrecorded | [`core/src/session/mod.rs:4377-4409`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4377-L4409), [`core/src/compact_remote.rs:289-308`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/compact_remote.rs#L289-L308) | uro-spce coverage diagnostics | Extends |
| `compacted.latest_token_usage_record` is a copy of an earlier record | [`history/src/lib.rs:196-202`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs#L196-L202) | uro-y2qj: never emit it as an observation | Extends |
| `token_count` also emitted for rate-limit-only, compaction-estimate and context-full updates with synthetic `last`/`total` | [`core/src/session/mod.rs:4447-4536`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L4447-L4536), [`protocol/src/protocol.rs:2291-2314`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2291-L2314) | uro-spce counter-epoch rule | Extends; explains the decreasing total |
| `info: null` means no usage recorded yet in the session | [`core/src/session/turn.rs:1489-1494`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L1489-L1494), [`core/src/session/turn.rs:2631-2636`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2631-L2636) | uro-y2qj: limit observation only | Resolves a squares review question |
| Rate-limit snapshot is session-latest, repeated per event, one `limit_id` at a time, with carried-forward fields | [`core/src/state/session.rs:278-289`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L278-L289), [`core/src/state/session.rs:388-411`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/state/session.rs#L388-L411) | uro-eamm; design §3.1 (provider limit observation) | Extends |
| `rate_limits` has `limit_id`, `limit_name`, `credits`, `individual_limit`, `spend_control_reached`, `plan_type`, `rate_limit_reached_type` | [`protocol/src/protocol.rs:2323-2391`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2323-L2391) | uro-eamm; uro-um7n plan terms | Extends |
| Served model not persisted; `turn_context.model` is requested | [`core/src/session/turn.rs:2589-2601`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn.rs#L2589-L2601), [`rollout/src/policy.rs:163`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/policy.rs#L163) | uro-wuby: label model basis `requested` | Extends |
| `CODEX_THREAD_ID` is the subagent’s own ID; `CODEX_SESSION_ID` is root | [`core/src/unified_exec/process_manager.rs:1370-1377`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/unified_exec/process_manager.rs#L1370-L1377), [`core/src/exec_env.rs:40-50`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/exec_env.rs#L40-L50) | uro-20ck `--current` | Resolves |
| Hooks get no `CODEX_*` identity variables; they replay the Codex process environment | [`hooks/src/registry.rs:73-82`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/registry.rs#L73-L82), [`hooks/src/engine/command_runner.rs:420-425`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/engine/command_runner.rs#L420-L425) | uro-20ck: `--current` from a hook must use `--hook-input` | Extends |
| Hook `session_id` is the root session; `agent_id` is the child thread; `SubagentStop.transcript_path` is the parent’s | [`core/src/hook_runtime.rs:151-158`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L151-L158), [`core/src/hook_runtime.rs:385-444`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/hook_runtime.rs#L385-L444) | design §6.2 (`--hook-input` check) | Corrected the plan’s earlier check |
| File-name timestamp and date directories are local time | [`rollout/src/recorder.rs:1635-1657`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1635-L1657) | uro-20ck discovery; never derive time from names | Extends |
| No rotation; resume appends; revert and paginated forks add files; archive renames to flat directory | [`rollout/src/recorder.rs:1924-1935`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1924-L1935), [`thread-store/src/local/revert_thread.rs:15-18`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/revert_thread.rs#L15-L18), [`thread-store/src/local/archive_thread.rs:80-119`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/archive_thread.rs#L80-L119) | uro-xm48; design §3.6 `src-` ID | Extends |
| Compression and in-place migration exist, both default off | [`features/src/lib.rs:1112-1129`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/features/src/lib.rs#L1112-L1129), [`thread-store/src/local/rollout_migration.rs:1-9`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration.rs#L1-L9) | uro-xm48, uro-gnqj: replacement handling | Extends |
| Subagent forks drop `token_usage_record`; legacy destinations keep parent `token_count` and foreign `session_meta` | [`core/src/agent/control/spawn.rs:65-105`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105), [`core/src/agent/control/spawn.rs:1007-1066`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L1007-L1066) | uro-spce replay rule | Confirms and extends |
| Legacy user forks copy parent records including `token_usage_record` | [`core/src/session/mod.rs:1513-1519`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L1513-L1519) | uro-spce: dedupe by `response_id`, owner = `record.thread_id` | Extends |
| `thread_settings_applied.thread_id` marks the child boundary (0.152+) | [`protocol/src/protocol.rs:2183-2191`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2183-L2191) | uro-spce legacy replay rule | Extends the squares review |
| `session_meta.git.branch` exists | [`protocol/src/protocol.rs:3334-3348`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3334-L3348) | uro-d135 `--group-by branch` (observed per thread) | Corrects the squares review |
| Parallel guardian reviews and `--ephemeral` threads write no rollout | [`core/src/guardian/review_session.rs:787-797`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/guardian/review_session.rs#L787-L797), [`core/src/session/session.rs:855-858`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/session.rs#L855-L858) | uro-d135 coverage notes | Extends |
| SQLite state names, `threads` and `thread_spawn_edges` schema; paginated metadata SQLite-authoritative | [`state/src/sqlite.rs:29-34`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/src/sqlite.rs#L29-L34), [`state/migrations/0021_thread_spawn_edges.sql:1-8`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/state/migrations/0021_thread_spawn_edges.sql#L1-L8), [`thread-store/src/local/thread_rollout_resolver.rs:91-94`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/thread_rollout_resolver.rs#L91-L94) | uro-20ck optional hint | Extends the squares review |

### Codex Reuse Table

Codex is Apache-2.0, so ported types and logic keep the license text, a statement of
changes and the `NOTICE` attribution.

| Item | Reuse mode | Bead |
| --- | --- | --- |
| `TokenUsage`, `TokenUsageInfo`, `TokenCountEvent`, `TokenUsageRecord`, `RateLimitSnapshot`, `RateLimitWindow`, `CreditsSnapshot` definitions [`protocol/src/protocol.rs:2215-2391`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2215-L2391) | Port types with Apache-2.0 notice, keeping unknown fields in a raw map | uro-y2qj |
| `SessionMeta`/`SessionMetaLine` with the missing-`session_id` fill, `SessionSource`, `SubAgentSource`, `ThreadSource` [`protocol/src/protocol.rs:2740-2840`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2740-L2840), [`protocol/src/protocol.rs:3034-3171`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3034-L3171) | Port types (drop `ts`/schemars derives) | uro-y2qj, uro-20ck |
| `RolloutFileName::parse` (plain and `_<rollout_id>` names) [`rollout/src/rollout_file_name.rs:39-60`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/rollout_file_name.rs#L39-L60) | Port code, fixing the local-time label | uro-20ck |
| Value-first line decoding and first-`session_meta` rule [`rollout/src/lib.rs:39-73`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/lib.rs#L39-L73), [`rollout/src/recorder.rs:1026-1089`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1026-L1089) | Port logic | uro-y2qj |
| Legacy normalizers (RFC 3339 `resets_at`, retired events, `ghost_snapshot`) [`thread-store/src/local/rollout_migration/line_parser.rs:33-135`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/rollout_migration/line_parser.rs#L33-L135) | Port logic and tests | uro-y2qj |
| Plain-over-compressed discovery and zstd reading [`rollout/src/compression.rs:144-192`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/compression.rs#L144-L192), [`rollout/src/seekable_reader_tests.rs:11-79`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/seekable_reader_tests.rs#L11-L79) | Port logic; adapt multi-frame zstd tests | uro-20ck, uro-gnqj |
| `forked_from_ordinal_exclusive` inference for older files [`rollout/src/metadata.rs:117-135`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/metadata.rs#L117-L135) | Port logic | uro-spce |
| Per-variant wire shapes [`history/src/tests.rs:393-518`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/tests.rs#L393-L518) and `rollout/src/tests.rs:52-74` | Adapt fixtures | uro-obx5 |
| Token scenarios: resume records, null info, context-full, compaction estimate, subagent non-inheritance (see [Codex Tests and Fixtures](#codex-tests-and-fixtures)) | Adapt fixtures as synthetic JSONL goldens | uro-obx5, uro-spce |
| Exec event enum and `Usage` [`exec/src/exec_events.rs:8-133`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/exec/src/exec_events.rs#L8-L133) | Port types | uro-y2qj |
| Hook input structs [`hooks/src/schema.rs:278-638`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/hooks/src/schema.rs#L278-L638) and generated schemas `codex-rs/hooks/schema/generated/` | Port types; format fact | uro-20ck |
| SQLite migrations list | Format fact only | uro-20ck |

### Codex Pitfalls

1. **Summing `turn.completed.usage`** across a resumed thread’s exec captures counts
   earlier runs again; subtract the previous capture’s total for the same `thread_id`,
   or prefer rollout records.
2. **Double-owning an exec capture and its rollout:** same `thread_id`, no shared
   request key. The rollout owns usage; the capture is a check that can differ because of
   subagents (excluded from exec), dropped notifications, and failed or interrupted
   turns (no usage in exec).
3. **`compacted.latest_token_usage_record`** looks like a record but is a copy.
4. **Copied `token_usage_record` in legacy user forks** carries the parent’s `thread_id`
   and `response_id`; a fork’s `thread_token_usage` and cumulative `token_count`
   continue the parent’s counters, so neither is the fork’s own usage.
5. **Legacy subagent `token_count` replay:** the child file repeats the parent’s
   cumulative events, and the child’s own counter starts from the parent’s total.
6. **Synthetic `last_token_usage`:** compaction estimates and context-full fills have
   zero input and output with nonzero `total_tokens`; ccusage’s “use `last` when the
   total advances” rule still counts a context-full fill as `window - previous_total`
   tokens.
7. **Repeated rate-limit snapshots** in every `token_count` are one observation, not
   many; carried forward `plan_type` or `credits` may be stale; other `limit_id` buckets
   in the same response are lost.
8. **Hook identity:** `session_id` is the root; comparing it with the transcript’s
   `session_meta.id` fails inside subagents.
   `SubagentStop.transcript_path` is the parent file.
9. **Nested Codex hooks** inherit the outer Codex’s `CODEX_THREAD_ID`.
10. **Path-based source identity breaks** on archive (rename to flat directory),
    compression (`.jsonl` to `.jsonl.zst`) and migration (in-place rewrite with
    different bytes and ordinals).
11. **Migration can remove usage records** of rolled-back turns and of a legacy
    subagent’s earlier history, so a cache built before migration can hold usage the
    rewritten file lacks.
    *(2026-09-15: urollup’s capture store keeps the pre-migration entry as a retained
    version under its own `src-` ID, so the dropped usage still counts once; see
    [design §2.5](../../urollup-design.md#25-capture-store-and-cache) and
    [Decision 8](../../urollup-design.md#decision-8-capture-store-and-cache).)*
12. **Record `timestamp` is write time:** copied history gets the child’s time; buckets
    by line time must skip copied prefixes.
13. **File-name time is local**, date directories are local, and a thread stays in its
    creation directory across later resumes.
14. **Unobserved usage:** ephemeral threads, parallel guardian reviews, legacy remote
    compaction, and responses without `usage`.
15. **SQLite `tokens_used`** is the last cumulative `total_tokens`, which includes
    inherited fork totals and context-full fills.

### Codex Still Unverified

- Behavior in a running process (all conclusions are source-only).
- Whether `cache_write_input_tokens` is a subset of `input_tokens`.
- Usage semantics for non-OpenAI `model_provider` values (Ollama, LM Studio, custom
  providers).
- Which early release first wrote `info: null`, and pre-0.152 subagent replay details.
- Whether Codex Desktop and IDE clients request `paginated` history (TUI since 0.147,
  exec since 0.148), and their `source`/`originator` values.
- Whether memory-consolidation threads link to a parent thread.
- Codex signals in cloud environments.
- Exact migration behavior for usage records (read from module docs, not traced line by
  line).

The portable brief’s
[Next Steps](research-2026-09-13-portable-agent-usage.md#next-steps) tracks these items.

## ccusage Findings

ccusage was reviewed at
[`bd7f89b`](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1)
(tag `v20.0.20`, 2026-08-15). Citations are `ccusage/<path>:<lines>` at that commit
unless a later commit is named; commits on `main` after the release, up to `95bbc41`
(2026-09-14), are cited by hash as evidence that a 20.0.20 behavior was a bug.

### ccusage Top Findings

1. **ccusage has no single reconciliation path.** Claude `daily` and the other Claude
   reports use two separately written parsers and dedupers that disagree (agent-progress
   records, index refresh, tie-breaks).
   Codex daily, session, table, JSON and `--since` runs use different dedupe keys.
   urollup’s one-ledger design is the right correction, and the feature matrix must name
   the ccusage command path for every comparison.
2. **Codex replay handling is value matching plus a timing heuristic.** It compares a
   child’s leading usage events with the parent’s usage stream up to the child’s
   `session_meta` timestamp, then falls back to skipping a burst of events less than 1 s
   apart. It reads neither `token_usage_record`, `subagent_history_start_ordinal`,
   `rate_limits`, `turn_context.effort` nor `.jsonl.zst`, at 20.0.20 or at `main` commit
   `95bbc41`.
3. **Several records are dropped silently.** A Claude assistant record whose tool input
   has a nested `"id":null` (or `model`, `cwd`, `version` and other names) loses its
   usage (confirmed with a synthetic probe).
   Timestamps with other than 0 or 3 fractional digits are dropped for Claude and abort
   the whole Codex report.
4. **Pricing is float, fuzzy, network-refreshed and inconsistent across adapters.**
   Default runs fetch LiteLLM `main` at runtime; missing cache rates default to
   `input × 1.25` and `input × 0.1` for every provider; LiteLLM long-context rates are
   applied marginally per token category, models.dev rates switch the whole request;
   unpriced tokens are `0` in JSON with only a stderr warning.
5. **Most reusable pieces are small and generic:** the ordered size-balanced parallel
   file reader, the `memmem` line prefilter, ANSI-aware terminal width and the
   `fs_fixture!` macro.
   The most valuable asset is the test corpus: about 600 `#[test]` functions, many of
   them edge cases worth porting as urollup fixtures (Codex fork and legacy subagent
   replay, repeated snapshots, sidechain replays, advisor iterations, gateway message-ID
   reuse).

### ccusage Architecture and Discovery

**Crate layout.** `rust/Cargo.toml` has two glob members, `adapters/*` and `crates/*`
([`ccusage/rust/Cargo.toml:1-7`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/Cargo.toml#L1-L7)):

- `adapters/`: 16 agent crates (amp, claude, codebuff, codex, copilot, droid, gemini,
  goose, grok, hermes, kilo, kimi, openclaw, opencode, pi, qwen) plus `common` (file
  walking, parallel reads, lenient JSONL helpers, shared agent tables).
  Each follows a documented module shape: `paths.rs`, `parser.rs`, `loader.rs`,
  `report.rs`, `types.rs`
  ([`ccusage/rust/adapters/README.md:13-22`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/README.md#L13-L22)).
- `crates/ccusage`: the binary; command dispatch, `blocks`, statusline and the HTTP
  fetcher.
- `crates/ccusage-core`: shared types, pricing, cost, dates, summaries, JSON output,
  progress.
- `crates/ccusage-adapter-all`: the unified `all` report, also the default command.
- `crates/ccusage-cli`, `ccusage-cli-parser`: argument types and a hand-written parser
  with generated help; `ccusage-config`: `ccusage.json` and its schema;
  `ccusage-terminal`: tables; `ccusage-test-support`: fixtures and environment guards.

**No adapter trait.** Each adapter exposes a `run(AgentCommandArgs)` function and ad hoc
`load_*` functions, and the binary dispatches with a `match` on the subcommand
([`ccusage/rust/crates/ccusage/src/main.rs:25-64`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L25-L64)).
The unified loader registers agents as a hard-coded vector of
`AgentLoadSpec { index, agent, load: Box<dyn FnOnce> }`, runs them on scoped threads and
sorts results by index
([`ccusage/rust/crates/ccusage-adapter-all/src/loader.rs:100-121`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-adapter-all/src/loader.rs#L100-L121),
[`382-440`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-adapter-all/src/loader.rs#L382-L440)).
Adapters reach core through a glob `use ccusage_core::*`
([`ccusage/rust/adapters/claude/src/lib.rs:1-2`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L1-L2)).
Core stays free of TLS because the binary injects the HTTP client with
`set_json_fetcher`
([`ccusage/rust/crates/ccusage-core/src/pricing.rs:2149-2176`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L2149-L2176)).

**Detection.** An agent is “detected” when its loader returns rows, or its `has_data()`
finds a store
([`ccusage/rust/crates/ccusage-adapter-all/src/loader.rs:442-452`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-adapter-all/src/loader.rs#L442-L452),
[`593-611`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-adapter-all/src/loader.rs#L593-L611)).
There is no content-based dialect detection at file level.
Inside the Codex adapter each line is classified separately as a rollout (`Session`) or
saved `codex exec` (`Headless`) record
([`ccusage/rust/adapters/codex/src/parser.rs:470-508`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L470-L508)),
so one file can contribute through both paths.

**Default paths and variables** (all comma-separated lists, a ccusage convention the
agents themselves do not define):

| Agent | ccusage roots | Variable | Notes |
| --- | --- | --- | --- |
| Claude | `$XDG_CONFIG_HOME/claude` (else `~/.config/claude`), then `~/.claude`, each only if `projects/` exists | `CLAUDE_CONFIG_DIR` | Accepts a config dir or its `projects/` dir; when set, defaults are dropped and no valid path is an error ([`ccusage/rust/adapters/claude/src/paths.rs:12-52`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/paths.rs#L12-L52)) |
| Codex | `$CODEX_HOME/sessions` and `archived_sessions`; the home itself if neither exists | `CODEX_HOME` | Files with the same path relative to their directory count once per home, active copy first ([`ccusage/rust/adapters/codex/src/paths.rs:20-116`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/paths.rs#L20-L116)) |
| Pi | `~/.pi/agent/sessions` | `PI_AGENT_DIR` (points at the sessions dir, no `~` expansion) | Not Pi’s `PI_CODING_AGENT_DIR` or `PI_CODING_AGENT_SESSION_DIR`; extra named stores from `ccusage.json` ([`ccusage/rust/adapters/pi/src/paths.rs:5-46`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/pi/src/paths.rs#L5-L46)) |
| Others | See each `rust/adapters/<agent>/README.md` “Data source” | `AMP_DATA_DIR`, `OPENCODE_DATA_DIR`, `GEMINI_DATA_DIR`, `GROK_HOME` and others | Out of urollup scope |

File walking uses `read_dir` entry types, so symlinked files and directories are
silently skipped, and only the `jsonl` extension is collected
([`ccusage/rust/adapters/common/src/lib.rs:14-34`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/common/src/lib.rs#L14-L34)).

### ccusage Claude Parsing

**Record acceptance**
([`ccusage/rust/adapters/claude/src/lib.rs:235-349`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L235-L349)):
read the whole file; keep lines containing the compact bytes `"usage":{`; reject any
line with a forbidden `:null` field anywhere in it (below); deserialize a typed
`UsageEntry`; require a parseable `timestamp`; require `version` to look like `N.N.N`
when present and `sessionId`, `requestId`, `message.id` and `message.model` to be
non-empty when present
([`414-453`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L414-L453)).
Unreadable files and malformed lines are skipped with no count.
`message.model == "<synthetic>"` keeps its tokens but gets no model;
`usage.speed == "fast"` renames the model with a `-fast` suffix
([`282-290`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L282-L290)).

**Cache writes.** `cache_creation_token_count()` uses
`cache_creation.ephemeral_5m_input_tokens + ephemeral_1h_input_tokens` whenever the
`cache_creation` object exists, and ignores `cache_creation_input_tokens` then
([`ccusage/rust/crates/ccusage-core/src/types.rs:28-57`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/types.rs#L28-L57)).

**Advisor iterations.** Records whose `message.usage.iterations` contain
`advisor_message` items produce one extra entry per item under the item’s own `model`,
with message ID `<id>:advisor:<n>` and no recorded cost
([`ccusage/rust/adapters/claude/src/lib.rs:306-346`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L306-L346),
[`351-404`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L351-L404)).
The fixture shows why: top-level usage equals the sum of the `message` iterations and
excludes the advisor item
([`ccusage/rust/crates/ccusage/src/main.rs:320-343`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L320-L343)).

**Dedupe at 20.0.20**
([`ccusage/rust/adapters/claude/src/lib.rs:98-233`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L98-L233)).
These details extend the portable brief’s summary:

- Files are sorted by path string and read in parallel, then deduped sequentially in
  that order, so the parent `<session>.jsonl` precedes its `<session>/subagents/` files
  ([`ccusage/rust/crates/ccusage/src/main.rs:207-218`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L207-L218)).
- The exact key is a 64-bit Fx hash of `(message.id, requestId)`, with collisions
  resolved by comparing the stored fields.
  A missing `requestId` is part of the key as `None`, so requestless copies dedupe by
  message ID alone
  ([`main.rs:281-301`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L281-L301)).
- **Entries without `message.id` are never deduplicated**
  ([`lib.rs:147`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L147),
  [`185-192`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L185-L192)).
- The sidechain fallback matches on `message.id` alone when either the candidate or the
  kept entry has `isSidechain: true`, for `/btw` logs that replay parent messages under
  new request IDs
  ([`157-170`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L157-L170);
  [`ccusage/rust/adapters/claude/src/README.md:19-31`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/README.md#L19-L31)).
- **Replacement order:** a non-sidechain copy beats a sidechain copy; then the larger
  `input + output + cache_creation + cache_read` total wins; then a copy with `speed`
  beats one without; otherwise the first copy stays
  ([`118-140`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L118-L140)).
  The winning record is kept whole, including its timestamp, date bucket and recorded
  `costUSD`, so a request can move across a day boundary when a later block record wins.
- `--project` filters entries before dedupe and restricts discovery to that project
  directory
  ([`103-108`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L103-L108);
  [`paths.rs:54-68`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/paths.rs#L54-L68)),
  so filtered totals are not a slice of unfiltered totals when history is copied across
  project directories.

**The second Claude pipeline.** `ccusage daily` and the daily base of `all` use
`load_daily_summaries` in `daily.rs`; `monthly`, `weekly`, `session`, `blocks` and
statusline use `load_entries` in `lib.rs`
([`ccusage/rust/crates/ccusage/src/commands/mod.rs:31-152`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L31-L152)).
They differ:

- `daily.rs` also parses `type: "progress"` lines whose `data.message` embeds a
  subagent’s assistant message with `message.id`, `requestId`, `usage` and `isSidechain`
  ([`ccusage/rust/adapters/claude/src/daily.rs:140-190`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L140-L190));
  `lib.rs` cannot deserialize those lines, so a progress-only subagent counts in `daily`
  and not in `monthly`.
- `daily.rs` does not re-index the kept entry when a candidate replaces it
  ([`daily.rs:426-431`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L426-L431),
  compare
  [`lib.rs:174-183`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L174-L183)).
  If a sidechain replay is seen before its parent’s records, the first parent record
  replaces it unindexed and the next parent record is pushed as a new entry, double
  counting. Commit
  [`a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84)
  added the missing re-index.
- `daily.rs` adds a recorded-cost tie-break
  ([`daily.rs:443-462`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L443-L462)).

**Session ID change after the release.** The change is
[`a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84)
(“fix(claude): scope message dedupe by session”, #1661, 2026-08-29); commit `1b4f423` is
a LiteLLM snapshot bump that includes it.
It adds the effective session ID (serialized `sessionId`, else the path-derived ID) to
every key, requires equal timestamps for sidechain matches in `load_entries`, and adds
the timestamp to requestless daily keys.
Its fixture uses a gateway that reuses message ID `ocgo` with no `requestId` across
sessions; copied transcripts that keep the original serialized `sessionId` still
collapse. The daily and regular loaders now disagree on timestamps by design (README
lines added in that commit).

**Sessions and projects** come from the path: the first component under `projects/` is
the project, and the session is the file stem, or the directory above `subagents/`
([`ccusage/rust/adapters/claude/src/paths.rs:91-153`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/paths.rs#L91-L153)).
A file under `<session>/subagents/workflows/<workflow>/` falls through to the generic
branch and becomes a session named after the workflow directory.

**Provider limits.** The only limit signal read is API-error text
`Claude AI usage limit reached|<epoch seconds>` on `isApiErrorMessage: true` records,
used for `blocks`
([`ccusage/rust/adapters/claude/src/lib.rs:532-557`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L532-L557)).
`quotaLimits` is not read.

### ccusage Codex Parsing

**Streaming parse**
([`ccusage/rust/adapters/codex/src/parser.rs:156-274`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L156-L274)):
a 128 KiB `BufReader`, one line at a time.
Lines are prefiltered by byte markers, then deserialized with borrowed `Cow` fields and
lossy visitors that turn unexpected types into absent or zero
([`ccusage/rust/adapters/codex/src/types.rs:93-300`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L93-L300),
[`396-480`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L396-L480)).

- **Session ID** is the file path relative to the source directory without `.jsonl`
  ([`parser.rs:673-685`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L673-L685)),
  not `session_meta.payload.id`. A thread in both `sessions/YYYY/MM/DD/` and a
  differently laid out `archived_sessions/` gets two session IDs and escapes the
  relative-path file dedupe.
- **Model** comes from the event (`payload` or `info` `model`, `model_name` or
  `metadata.model`), else the latest `turn_context`, else `gpt-5` flagged `isFallback`
  ([`parser.rs:583-612`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L583-L612)).
  The placeholder `codex-auto-review` is replaced by the newest model released on or
  before the event date from an embedded table
  ([`614-627`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L614-L627);
  `codex-auto-review-fallbacks.json`). `turn_context.effort` is not read.
- **Service tier** comes from `event_msg` `thread_settings_applied`
  `thread_settings.service_tier`: `default` or `standard` is Standard, `priority` or
  `fast` is Fast, any other value clears the tier, and a settings event without the key
  keeps the previous tier
  ([`302-316`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L302-L316),
  [`459-468`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L459-L468)).
  Copies with conflicting tiers resolve to Standard
  ([`types.rs:20-36`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L20-L36)).

**Cumulative counters**
([`parser.rs:320-367`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L320-L367)).
Beyond the core rule that the portable brief describes:

- `info: null`, or `info` with neither usage object, produces no event.
- With `total_token_usage` present and unchanged from the previous event,
  `last_token_usage` is ignored and the zero difference is skipped.
  With the total advanced, `last_token_usage` is trusted even when it disagrees with the
  difference, so a missing line loses the gap.
  With only a total, the saturating difference is used, so a counter reset yields zeros,
  not a new epoch
  ([`1063-1084`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L1063-L1084)).
- The previous total updates even for events the replay filter later drops, so the
  baseline stays right after a skipped replay
  ([`ccusage/rust/adapters/codex/src/loader.rs:1417-1496`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1417-L1496)).
- Events whose input, cached, output and reasoning counts are all zero are dropped even
  with a non-zero total
  ([`parser.rs:339-345`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L339-L345)).
- `cached_input_tokens` is silently clamped to `input_tokens`
  ([`parser.rs:361`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L361)).
- A recorded `total_tokens` of 0 is treated as missing and derived as input plus output,
  never adding reasoning
  ([`types.rs:262-300`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L262-L300)).
- `cache_write_input_tokens` is ignored at 20.0.20, so cache writes are priced as
  uncached input. Commit
  [`15b3bef`](https://github.com/ccusage/ccusage/commit/15b3bef85b1e0d440ca98e345b4fb5610a41a195)
  (#1663) later reads it as a subset of `input_tokens`, clamped to `input - cached`.

**`token_usage_record`** is not parsed at 20.0.20 or at `95bbc41`. Such a line contains
`"usage":`, so the classifier routes it to the `codex exec` branch, which looks only for
a top-level `usage`, `data.usage`, `result.usage` or `response.usage` and finds none
([`parser.rs:501-506`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L501-L506),
[`781-804`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L781-L804)).
It is ignored today only by accident of shape.

**Saved `codex exec` output** is read by the same adapter: any line with a usage object
and no rollout type marker.
Missing timestamps fall back to the file’s modification time
([`parser.rs:370-457`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L370-L457),
[`1050-1061`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L1050-L1061));
field spellings `prompt_tokens`, `completion_tokens`, `cached_tokens`,
`cache_read_input_tokens` and `reasoning_tokens` are accepted
([`types.rs:234-260`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L234-L260);
[`loader.rs:521-587`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L521-L587)).

**Fork and subagent replay plan**
([`ccusage/rust/adapters/codex/src/replay.rs:31-111`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs#L31-L111),
[`224-261`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs#L224-L261)):

1. Read only the first line of every file.
   If it is `session_meta`, record `payload.id`, the parent (`forked_from_id`, else
   `source.subagent.thread_spawn.parent_thread_id`) and the line timestamp.
   The first file with an ID wins; a self-parent is ignored.
2. Fully parse every referenced parent file without replay filtering and keep its usage
   events and timestamps.
   Parents are therefore parsed twice.
3. For a child, the replay prefix is the parent’s events up to the first one later than
   the child’s `session_meta` timestamp.
4. While parsing the child, each leading event is compared for exact equality of its
   five counters with the next prefix event and skipped on a match
   ([`parser.rs:177-221`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L177-L221)).
5. If the first event does not match (parent missing, or history compacted), and the
   child’s first two usage events are at most 1,000 ms apart, skip every leading event
   within 1,000 ms of the previous one
   ([`parser.rs:84-149`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L84-L149)).
   The comment records measurements: replay bursts of 10–40 ms, followed by 5.8–15.3 s
   before the child’s own first turn.

Legacy subagent replay is handled by step 5 only: the fixtures model a child with its
own `session_meta`, a foreign parent `session_meta` and compressed replayed
`token_count` events stamped at the creation time
([`loader.rs:1498-1656`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1498-L1656)),
matching squares’ observations.
A compressed replay of a single event is not detected, and a real child whose first two
usage events are under a second apart loses them.
`subagent_history_start_ordinal` is not used.

The adapter README documents MultiAgent V2 boundaries (`task_started` ends the replayed
prefix; `inter_agent_communication_metadata`, earlier `inter_agent_communication`, with
`trigger_turn: true` starts the child turn) and says both spellings are matched
([`ccusage/rust/adapters/codex/src/README.md:30-36`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/README.md#L30-L36)).
No code references either marker at 20.0.20 or `95bbc41`.

**Cross-file event dedupe** keys differ by path:

- `load_codex_events`: `(timestamp string, model before alias, input, cached, output,
  reasoning, total)` across all files
  ([`loader.rs:149-172`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L149-L172)).
- `load_groups`: `(parsed timestamp, model after alias, counters)` plus the session ID
  only for `session` reports, in mutex-sharded maps
  ([`ccusage/rust/adapters/codex/src/aggregate.rs:23-35`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L23-L35),
  [`479-554`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L479-L554)).
- The unified loader uses `load_groups` without `--since`/`--until` and
  `load_codex_events` with them
  ([`ccusage/rust/crates/ccusage-adapter-all/src/loader.rs:625-655`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-adapter-all/src/loader.rs#L625-L655)),
  and `codex` table output with one source takes a different aggregation path from JSON
  output
  ([`aggregate.rs:60-69`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L60-L69)).

Two genuinely distinct requests in different sessions with the same millisecond
timestamp, model and counts merge in daily totals but not in session totals.

### ccusage Pi and Other Adapters

ccusage 20.0.20 ships 14 further adapters: Amp (thread JSON), Codebuff, Copilot
(OpenTelemetry file exporter JSONL), Droid, Gemini CLI, Goose (SQLite), Grok, Hermes
(SQLite), Kilo (SQLite), Kimi, OpenClaw, OpenCode (SQLite plus legacy JSON), Pi and
Qwen; `main` has since added ZCode and Antigravity.
Only Pi and, from 2026-09-15, Gemini CLI are in urollup’s scope; the Gemini adapter has
its own section, [ccusage Gemini CLI parsing](#ccusage-gemini-cli-parsing).

**Pi at 20.0.20**
([`ccusage/rust/adapters/pi/src/parser.rs:162-287`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/pi/src/parser.rs#L162-L287),
[`374-395`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/pi/src/parser.rs#L374-L395)):

- Keeps `type: "message"` lines with `message.role == "assistant"` and a `usage`; reads
  `input`, `output`, `cacheRead`, `cacheWrite`, `totalTokens` and `cost.total`; ignores
  `id`, `parentId`, `responseId`, `provider`, `reasoning` and `cacheWrite1h`.
- Session ID is the file stem after the first `_`; project is the directory under
  `sessions/`.
- Dedupe key is
  `store:project:session:timestamp:model:counters:extra:cost-as-float-string`, so forks
  (new file, new stem) double count.
  Commit
  [`809eeb6`](https://github.com/ccusage/ccusage/commit/809eeb6d52a2c7d13b9c65e10d4106109247390c)
  (#1662) later suppresses a leading child prefix matching the parent’s active
  root-to-leaf branch, matching timestamp, model, tokens and cost, and follows the
  header `parentSession`.
- When `totalTokens` exceeds the known parts, the remainder becomes `output_tokens` if
  output was zero, else a hidden “extra” count
  ([`ccusage/rust/crates/ccusage-core/src/utils.rs:21-37`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/utils.rs#L21-L37)).
- The model label is decorated as `[pi] <model>` and priced through the fuzzy matcher on
  that label, regardless of the recorded provider.

Relevance of the rest: learn only.
Their READMEs are a quick index of storage locations, and the Grok README documents a
mapping (`inputTokens` includes cache reads, `costUsdTicks` are 1e-10 USD) that shows
how varied “input” semantics are
([`ccusage/rust/adapters/grok/README.md`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/grok/README.md)).
urollup’s scope still excludes them.

### ccusage Gemini CLI Parsing

*(Added 2026-09-15, when Gemini CLI entered urollup’s scope as a planned agent;
[design Decision 28](../../urollup-design.md#decision-28-gemini-cli-planned-support).)*
The adapter is small and, unlike the Claude and Codex ones, has no cross-file dedupe
([`ccusage/rust/adapters/gemini/src/parser.rs:115-219`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/gemini/src/parser.rs#L115-L219),
[`315-416`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/gemini/src/parser.rs#L315-L416),
[`paths.rs:5-42`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/gemini/src/paths.rs#L5-L42),
[`loader.rs:18-43`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/gemini/src/loader.rs#L18-L43)):

- **Discovery:** every `.json` and `.jsonl` file under `GEMINI_DATA_DIR`, a
  comma-separated list, else `~/.gemini/tmp`, walked recursively.
  It reads the `tmp` root rather than the Gemini home, ignores `GEMINI_CLI_HOME`, and
  sees every file in the bucket, not only `chats/`, so a prompt log, checkpoint or
  activity log is read and then ignored for lack of a usage shape.
- **Records:** a JSONL line is usage when `type == "gemini"` and it has `tokens`; a
  whole-file JSON document is read as a legacy record with a `messages` array, a single
  `gemini` record, or a `stats` object.
  `$set` and `$rewindTo` records are ignored, which is right for `$rewindTo`, since
  rewound requests were still served, and wrong for a `$set.messages` rewrite only
  because that record’s copies are nested under `$set`.
- **Dedupe:** by message `id`, keeping the last occurrence, **within one file**
  ([`parser.rs:174-201`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/gemini/src/parser.rs#L174-L201)).
  Nothing dedupes across files, so the copies Gemini CLI itself creates — the
  slug-bucket migration copy of the legacy `<sha256>` directory, a legacy `.json` file
  beside the `.jsonl` it was migrated into, and a `--session-file` import — count twice.
- **Tokens:** `input` is treated as excluding `cached` unless the record’s `total`
  equals `input + output + thoughts + tool`, in which case `cached` is subtracted from
  `input`; `tool` is then added to input, `thoughts` are priced as output through the
  hidden “extra” total, and `cache_creation` is always 0
  ([`parser.rs:400-416`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/gemini/src/parser.rs#L400-L416)).
  Gemini’s `promptTokenCount` always includes cached tokens, so any record whose `total`
  fails that equality — which the API reference suggests happens when
  `toolUsePromptTokenCount` is nonzero — counts its cached tokens twice, once as input
  and once as cache read.
- **Identity:** the session is the last `sessionId` seen in the file, else the file
  stem, so subagent files become their own sessions with no parent link; the project is
  the constant string `gemini`, so there is no project grouping; a record without
  `model` inherits the last model seen in the file, and an all-zero record is dropped.
- **Timestamps:** the record `timestamp`, else `created_at`, else the file’s mtime, so a
  record with an unparsable timestamp lands on the file’s modification date.
- **Later commits:** `main` through `d341949` changes no Gemini parsing; `c951e20` only
  separates the Antigravity source and `d39a09d` makes Gemini pricing timestamp-aware.

The writer-side facts these assumptions meet are in the portable brief’s
[Gemini CLI dialect facts](research-2026-09-13-portable-agent-usage.md#gemini-cli-dialect-facts),
and the differences above become parity ledger entries in the plan’s
[ccusage reconciliation harness](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#ccusage-reconciliation-harness).

### ccusage Pricing

**Loading**
([`ccusage/rust/crates/ccusage-core/build.rs:17-39`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/build.rs#L17-L39),
[`154-214`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/build.rs#L154-L214);
[`src/pricing.rs:630-741`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L630-L741)):

- The build embeds a LiteLLM snapshot (from `CCUSAGE_PRICING_JSON_PATH`, normally the
  `flake.lock` pin) reduced to Claude, Anthropic, GPT, OpenAI, Azure, ZAI and
  OpenRouter-OpenAI keys and nine price and context fields plus the fast multiplier,
  together with a committed models.dev snapshot and catalog trust rules, all deflated.
  `float_roundtrip` is enabled because default float parsing shifted prices by one ULP.
- At runtime: embedded LiteLLM, then 37 hard-coded entries (Claude, GLM, Kimi and
  others) that fill only missing keys
  ([`pricing.rs:1363-1877`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L1363-L1877)),
  then models.dev long-context tiers for entries without LiteLLM tier fields.
  **Unless `--offline`, it fetches LiteLLM from `main`** (not the pinned commit) and
  lets missing models fall back to the live models.dev API
  ([`51-53`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L51-L53),
  [`644-680`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L644-L680)).
  Pricing overrides from `ccusage.json` apply last.
- Missing LiteLLM cache rates default to `input × 1.25` (write) and `input × 0.1` (read)
  for every provider
  ([`719-722`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L719-L722));
  the Codex cost path instead uses the full input rate when the read rate was not
  explicit
  ([`ccusage/rust/adapters/codex/src/report.rs:187-219`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/report.rs#L187-L219)).

**Cache writes and tiers**
([`ccusage/rust/crates/ccusage-core/src/cost.rs:7`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/cost.rs#L7),
[`99-192`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/cost.rs#L99-L192)):

- One-hour cache writes cost `input × 2.0`, a hard-coded constant, and 5-minute writes
  the table’s write rate.
  Without the `cache_creation` breakdown, all writes are 5-minute.
- With a models.dev `long_context_threshold`, input plus cache reads plus cache writes
  above the threshold bill every category of the request at the long-context rate.
- With LiteLLM `*_above_200k_tokens` fields, each category is tiered **marginally and
  separately** at 200K: only that category’s tokens above 200K get the higher rate, and
  output is tiered by output count.
  This does not match whole-request tiering, and it applies to exactly the models
  LiteLLM publishes tiers for, because models.dev tiers fill only entries without them
  ([`pricing.rs:1296-1328`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L1296-L1328)).
- Codex requests are split into long-context buckets during aggregation by per-request
  input size (272K for OpenAI models from models.dev tiers)
  ([`ccusage/rust/adapters/codex/src/aggregate.rs:367-377`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L367-L377);
  [`pricing.rs:2032-2044`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L2032-L2044)).
- Fast mode multiplies cost by a per-model factor from LiteLLM’s
  `provider_specific_entry.fast` or an embedded override table (Opus 4.6 and 4.7 at 6×,
  Opus 4.8 at 2×, GPT-5.5 at 2.5×, GPT-5.4 and GPT-5.3-codex at 2×)
  ([`ccusage/rust/crates/ccusage-core/src/fast-multiplier-overrides.json`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/fast-multiplier-overrides.json)).
  For Codex rollouts with no recorded tier, `--speed auto` treats the unclassified usage
  as Fast if the user’s current `config.toml` sets `service_tier = "fast"`
  ([`ccusage/rust/adapters/codex/src/speed.rs:23-61`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/speed.rs#L23-L61);
  [`report.rs:131-152`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/report.rs#L131-L152)).

**Fuzzy matching**
([`pricing.rs:970-1103`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L970-L1103),
[`1946-2014`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L1946-L2014)):
exact key, then a small alias table (`gpt-5.6` to `gpt-5.6-sol`), then the longest key
that appears inside the model name or contains it at non-alphanumeric boundaries,
treating `.`, `@` and `-` as equal and refusing a following numeric version unless it is
an 8-digit date. An “exact-only” set keeps tier variants such as `-fast` or regional IDs
out of the scan, with several rounds of fixes in its tests.
`CCUSAGE_MODEL_ALIASES` rewrites model names globally, for display, pricing and Codex
dedupe keys
([`ccusage/rust/crates/ccusage-core/src/model_aliases.rs:13-37`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/model_aliases.rs#L13-L37)).
urollup’s price table instead matches models exactly or through listed aliases
([design §4.5](../../urollup-design.md#45-price-table)); the useful residue is the test
list of spellings a real matcher met.

**Floats.** Rates are `f64` per token, per-entry costs are summed as `f64` (Claude) or
computed from summed tokens (Codex), and JSON prints raw floats (`json_float`,
[`ccusage/rust/crates/ccusage-core/src/output.rs:385-395`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/output.rs#L385-L395)).
Prices have no effective dates at 20.0.20; commit
[`d39a09d`](https://github.com/ccusage/ccusage/commit/d39a09def5b4fd36369dfc9e710a916126497cba)
later threads event timestamps through cost functions for DeepSeek V4’s dated and
peak-hour schedules, the first time-aware rate.

**Cost modes.** `auto` (default) uses a record’s own `costUSD` (Claude) or `cost.total`
(Pi) when present and computes list price otherwise; `display` uses only recorded cost;
`calculate` only list price
([`cost.rs:19-33`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/cost.rs#L19-L33)).
One total can mix source estimates and list-price estimates.

**Unpriced models** cost 0. `ModelBreakdown.missing_pricing` is not serialized
([`ccusage/rust/crates/ccusage-core/src/types.rs:93-106`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/types.rs#L93-L106)),
so JSON consumers cannot tell unpriced from free; a warning goes to stderr
([`output.rs:338-383`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/output.rs#L338-L383)).

### ccusage Reports, Time and Output

- **Calendar buckets** are date strings formatted in `--timezone`, else the system zone
  ([`ccusage/rust/crates/ccusage-core/src/date_utils.rs:293-312`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L293-L312)).
  An unknown zone name silently falls back to the system zone.
- **`--since` and `--until`** are inclusive `YYYYMMDD` string comparisons against those
  dates, partial bounds allowed
  ([`date_utils.rs:190-230`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L190-L230)).
  `date_range_bounds_ms` resolves bounds to local midnight with DST handled by jiff,
  with a test for a short spring-forward day
  ([`date_utils.rs:212-241`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L212-L241)).
- **Weekly and monthly** are built from daily rows.
  Claude `weekly` defaults to Sunday with `--start-of-week`
  ([`ccusage/rust/crates/ccusage-cli-parser/src/parser.rs:500-510`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-cli-parser/src/parser.rs#L500-L510));
  Codex and `all` weeks start on Monday with no option
  ([`ccusage/rust/adapters/codex/src/aggregate.rs:334-339`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L334-L339);
  [`ccusage/rust/crates/ccusage-adapter-all/src/loader.rs:837-861`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-adapter-all/src/loader.rs#L837-L861)).
- **Session reports** group Claude entries by path-derived project and session, so
  subagent files fold into their parent’s row; with a date range they keep whole
  sessions whose last activity falls inside
  ([`ccusage/rust/crates/ccusage/src/commands/mod.rs:144-215`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L144-L215)),
  which commit
  [`b2809fa`](https://github.com/ccusage/ccusage/commit/b2809fa580962f39483a3a0e3fca937c74de7dcb)
  (#1664) later changed to clip totals to the window.
  `session --id` matches either the serialized or path session ID and reports raw
  `cache_creation_input_tokens`, not the breakdown sum
  ([`217-268`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L217-L268)).
- **`blocks`** is as [design §4.4](../../urollup-design.md#44-usage-windows) describes
  ([`ccusage/rust/crates/ccusage/src/blocks.rs:53-107`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/blocks.rs#L53-L107)).
- **JSON shapes** are camelCase with no schema version, currency, pricing basis or
  coverage. Claude rows carry `inputTokens`, `outputTokens`, `cacheCreationTokens`,
  `cacheReadTokens`, `totalTokens` (sum of the four), `totalCost`, `modelsUsed` and a
  `modelBreakdowns` array, under `daily`, `weekly`, `monthly` or `sessions` plus
  `totals`
  ([`output.rs:27-101`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/output.rs#L27-L101)).
  Codex rows carry non-cached `inputTokens`, the recorded `totalTokens`,
  `reasoningOutputTokens`, `costUSD` and a `models` map with `isFallback`
  ([`ccusage/rust/adapters/codex/src/report.rs:51-129`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/report.rs#L51-L129)).
  `totalTokens` therefore means different things per agent.
  `--no-cost` strips cost fields recursively and `--jq` pipes to an external `jq`
  process
  ([`output.rs:114-143`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/output.rs#L114-L143)).
- **Terminal rendering:** `SimpleTable` draws box borders, wraps multiline cells, fits
  the terminal width, compacts dates and measures ANSI-aware display width with
  `unicode-width`
  ([`ccusage/rust/crates/ccusage-terminal/src/table.rs`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-terminal/src/table.rs),
  [`width.rs:1-66`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-terminal/src/width.rs#L1-L66)).
  It prints straight to locked stdout
  ([`table.rs:58-65`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-terminal/src/table.rs#L58-L65))
  and does not strip control characters from log-supplied model or project names.
- `<synthetic>` and other model-less entries add to row totals but not to
  `modelBreakdowns`
  ([`ccusage/rust/adapters/claude/src/daily.rs:489-516`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L489-L516)),
  so breakdowns need not sum to the row.

### ccusage Performance

- **Parallel reads:** `read_files_parallel` sorts files by size, deals them greedily
  into `available_parallelism()` chunks of roughly equal bytes, reads each chunk on a
  scoped thread and returns results in the original file order
  ([`ccusage/rust/adapters/common/src/lib.rs:49-126`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/common/src/lib.rs#L49-L126)).
  Dedupe then runs sequentially, which keeps results independent of scheduling.
- **Claude reads each file whole** into memory (`fs::read`,
  [`ccusage/rust/adapters/claude/src/lib.rs:249`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L249)),
  splits lines with `memchr`, and skips non-usage lines with a SIMD `memmem` prefilter
  before typed `serde_json::from_slice`
  ([`ccusage/rust/crates/ccusage-core/src/fast.rs:17-104`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/fast.rs#L17-L104)).
  Every kept entry holds an owned `UsageEntry` clone in a global `Vec` until the report
  ends, so memory grows with the number of usage records.
- **Codex streams** lines and aggregates into per-worker period maps without a global
  event vector, with dedupe maps sharded by hash behind mutexes
  ([`ccusage/rust/adapters/codex/src/aggregate.rs:151-204`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L151-L204),
  [`479-529`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L479-L529)).
  The dedupe maps still grow with distinct events, and parent rollouts are parsed twice.
- **Pricing lookups** are memoized per model name, including misses
  ([`ccusage/rust/crates/ccusage-core/src/pricing.rs:970-1024`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L970-L1024)),
  and snapshots inflate once on first use.
- **Build:** release uses `opt-level = "z"`, fat LTO, one codegen unit and
  `panic = "abort"`
  ([`ccusage/rust/Cargo.toml:68-76`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/Cargo.toml#L68-L76));
  mimalloc only on musl
  ([`ccusage/rust/crates/ccusage/src/main.rs:21-23`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L21-L23)).
  urollup’s release profile keeps unwinding for `serve`
  ([design §8.2](../../urollup-design.md#82-engineering-conventions)), so the profile is
  not copied.
- **No log cache.** Every run rescans everything, and `load_entries` ignores `--since`
  while reading. Statusline performs up to three full Claude scans per render (session
  cost, today, blocks), mitigated by a per-session output cache in the shared temp
  directory, written non-atomically and invalidated by transcript mtime and a live-PID
  check
  ([`ccusage/rust/crates/ccusage/src/commands/mod.rs:318-383`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L318-L383),
  [`418-460`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L418-L460),
  [`563-572`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L563-L572),
  [`738-787`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L738-L787)).
  Commit
  [`527ec3a`](https://github.com/ccusage/ccusage/commit/527ec3a9cefa28391664b5c0c0ce1aa006264769)
  later skips Codex history outside the date window.
  urollup instead keeps a capture store from Phase 1 and reads it as a cache from Phase
  2 ([Decision 8](../../urollup-design.md#decision-8-capture-store-and-cache)).
- **Benchmarks:** `generate-large-fixture.ts` scales synthetic Claude and Codex trees to
  a real-world file-size profile of 3,142 files and 1.24 GiB: median 105 KB, p90 654 KB,
  p99 5.4 MB, maximum 87 MB
  ([`ccusage/apps/ccusage/scripts/generate-large-fixture.ts:8-19`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/apps/ccusage/scripts/generate-large-fixture.ts#L8-L19)).
  Its records are unrealistic for accounting: every Codex event has
  `total_token_usage == last_token_usage` with reasoning added into the total, and no
  Claude request is split into blocks
  ([`123-172`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/apps/ccusage/scripts/generate-large-fixture.ts#L123-L172)).
  PR comparisons run hyperfine with warmups and take median peak RSS from
  `/usr/bin/time` (`ccusage/apps/ccusage/scripts/compare_pr_performance/benchmark.clj`).

### ccusage Tests and Fixtures

- **Style:** unit tests next to code, synthetic JSONL written with `fs_fixture!` into
  `assert_fs` temp directories, `insta` JSON snapshots under `src/snapshots/`, and many
  tests asserting single-threaded and parallel runs agree.
  About 607 `#[test]` functions.
- **Environment:** `EnvVarGuard` mutates the process environment under a global mutex
  with `unsafe set_var`
  ([`ccusage/rust/crates/ccusage-test-support/src/lib.rs:13-79`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-test-support/src/lib.rs#L13-L79)).
  urollup injects environment and writers instead, so port the fixture macro, not the
  guard.
- **File fixtures:** `ccusage/apps/ccusage/test/fixtures/{claude,codex}` are three tiny
  synthetic files; `test-transcript.jsonl` is synthetic.
  The `statusline-test*.json` files are real hook inputs (see
  [Licenses and Fixture Privacy](#licenses-and-fixture-privacy)).
- **License for tests:** tests are MIT code; ported cases need the notice in the file
  header or a third-party notice file, plus source path and commit.

Test cases worth porting, as urollup fixtures with expected ledger results rather than
ccusage’s numbers where ccusage is wrong:

| Area | Test | Where |
| --- | --- | --- |
| Claude | Most complete duplicate wins; requestless duplicate | [`ccusage/rust/crates/ccusage/src/main.rs:257-301`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L257-L301) |
| Claude | Advisor iteration as separate model usage; daily includes it | [`main.rs:319-369`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L319-L369) |
| Claude | Agent-progress nesting and duplicate of subagent file | [`main.rs:425-484`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L425-L484) |
| Claude | Sidechain replay with new request ID; re-index on replacement | [`ccusage/rust/adapters/claude/src/lib.rs:690-782`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L690-L782) |
| Claude | Gateway reusing message IDs across sessions; copied transcript with same `sessionId` | commit [`a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84), `lib.rs` tests |
| Claude | Null-field rejection (port as a negative: urollup must keep the record) | [`lib.rs:638-656`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L638-L656) |
| Codex | Copied branch history with cumulative totals only | [`ccusage/rust/adapters/codex/src/loader.rs:445-519`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L445-L519) |
| Codex | Saved `codex exec` usage spellings and zero totals | [`loader.rs:520-587`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L520-L587) |
| Codex | Thread-spawn and forked replay; baseline after skipped replay; compressed replay across subagents | [`loader.rs:1042-1656`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1042-L1656) |
| Codex | Fork timestamp bound; rewritten burst; burst across a second tick; fork-local usage after burst; nested replays; self-parent; repeated snapshot inside burst | [`loader.rs:1657-1971`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1657-L1971) |
| Codex | Repeated `last_token_usage` with unchanged total | [`loader.rs:1972-2011`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1972-L2011) |
| Codex | Tier transitions, missing key keeps tier, unknown tier clears it | [`loader.rs:198-396`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L198-L396) |
| Codex | Total derivation without reasoning; saturating totals | [`ccusage/rust/adapters/codex/src/types.rs:489-570`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L489-L570) |
| Codex | Active and archived discovery | [`ccusage/rust/adapters/codex/src/paths.rs:124-251`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/paths.rs#L124-L251) |
| Codex | Per-request long-context split; dedupe after alias resolution | [`ccusage/rust/adapters/codex/src/aggregate.rs:883-1110`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L883-L1110) |
| Cost | Cache-write breakdown by duration; flat fallback; tiered cost | [`ccusage/rust/crates/ccusage-core/src/cost.rs:221-305`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/cost.rs#L221-L305); [`main.rs:137-142`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L137-L142) |
| Time | Zone formatting, offsets, local-midnight bounds across DST | [`main.rs:220-242`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L220-L242); [`date_utils.rs:411-426`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L411-L426) |
| Terminal | Narrow tables, ANSI truncation boundary, multiline titles | `ccusage/rust/crates/ccusage-terminal/src/snapshots/` |

### ccusage Reuse Table

Reuse modes follow the [conventions in Scope](#scope); ported ccusage code and tests
keep the MIT notice and source commit.

| Item | File:lines (pinned) | Mode | urollup bead or doc section | Notes and risks |
| --- | --- | --- | --- | --- |
| Ordered size-balanced parallel file reader | [`ccusage/rust/adapters/common/src/lib.rs:49-126`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/common/src/lib.rs#L49-L126) | Port code | uro-y2qj; design §8.3 | Replace `expect` panics with errors; add worker and memory limits; chunking by size, not a work queue, so one huge file still pins a worker |
| `memmem` line prefilter and `byte_lines` | [`ccusage/rust/crates/ccusage-core/src/fast.rs:17-104`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/fast.rs#L17-L104) | Port code | uro-y2qj; uro-gnqj | Use only as a hint that routes to full parsing; count every skipped or failed line in the snapshot manifest; markers must tolerate JSON whitespace (see `codex_line_type_flags`) |
| Whitespace-tolerant `"type"` value scanner | [`ccusage/rust/adapters/codex/src/parser.rs:510-563`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L510-L563) | Port logic | uro-y2qj (dialect detection) | Cheap record-type sniffing for Codex lines |
| ANSI-aware width and truncation; boxed table | [`ccusage/rust/crates/ccusage-terminal/src/width.rs:1-161`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-terminal/src/width.rs#L1-L161); [`table.rs`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-terminal/src/table.rs) | Port code | uro-d135 terminal format | Write to an injected writer; strip control characters from log strings; Markdown output must stay flowmark-stable |
| `fs_fixture!` macro and `Fixture` | [`ccusage/rust/crates/ccusage-test-support/src/lib.rs:81-142`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-test-support/src/lib.rs#L81-L142) | Port code | uro-phi8; uro-obx5 | Skip `EnvVarGuard`; inject environment |
| DST-safe local-midnight bounds | [`ccusage/rust/crates/ccusage-core/src/date_utils.rs:212-241`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L212-L241) | Port logic | uro-d135 (`--since`, `--until`, `--timezone`) | urollup uses half-open instants; reject unknown zones instead of falling back |
| Claude duplicate resolution (sidechain precedence, largest total, whole-record winner) | [`ccusage/rust/adapters/claude/src/lib.rs:118-233`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L118-L233); commit `a4b8420` | Port logic | uro-spce; design §3.4, §3.6 | Implement as group-then-resolve, independent of order; gateway reuse becomes ambiguous, not merged; entries without `message.id` need a fallback key |
| Advisor `iterations` usage | [`lib.rs:306-404`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L306-L404); fixture [`main.rs:319-369`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L319-L369) | Port logic; format fact | uro-y2qj; design §4.1 | One request with a second model’s usage, not a second request; price each iteration by its model |
| `progress` records embedding subagent messages | [`ccusage/rust/adapters/claude/src/daily.rs:140-190`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L140-L190) | Port logic; format fact | uro-y2qj; design §2.3 | Capture as observations of the subagent request; reconcile with subagent file by key |
| Usage-limit reset from API error text | [`lib.rs:532-557`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L532-L557) | Port logic | uro-y2qj; uro-eamm | Provider limit observation with basis “parsed from error text”; complements `quotaLimits` |
| Claude and Codex discovery rules | [`ccusage/rust/adapters/claude/src/paths.rs:12-68`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/paths.rs#L12-L68); [`ccusage/rust/adapters/codex/src/paths.rs:20-116`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/paths.rs#L20-L116) | Port logic | uro-y2qj; uro-20ck; design §2.1 | Adopt `XDG_CONFIG_HOME/claude` and a `projects/`-dir value; keep urollup’s single native path and `UROLLUP_*` lists; follow symlinks inside roots |
| Codex cumulative-to-delta rule | [`ccusage/rust/adapters/codex/src/parser.rs:320-367`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L320-L367) | Port logic; adapt tests | uro-y2qj; uro-spce; design §3.3, §3.4 | Add counter epochs and reset diagnostics; check `last` against the difference; keep `info: null` as a limit-only record |
| Codex usage field spellings and total derivation | [`ccusage/rust/adapters/codex/src/types.rs:234-300`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L234-L300) | Port logic | uro-y2qj (`codex-exec`) | Do not coerce wrong types to 0; record the spelling observed |
| Codex tier tracking | [`parser.rs:302-316`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L302-L316), [`459-468`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L459-L468) | Port logic; format fact | uro-y2qj; uro-wuby | Observed tier per request; unknown value leaves tokens unpriced; never infer from `config.toml` |
| Codex replay plan and burst heuristic | [`ccusage/rust/adapters/codex/src/replay.rs:31-111`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/replay.rs#L31-L111); [`parser.rs:84-221`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L84-L221) | Port logic (fallback only); adapt tests | uro-spce; uro-y2qj; design §3.3, §3.4 | Last resort after response IDs and `subagent_history_start_ordinal`; label exclusions `inferred`; combine with squares’ foreign-`session_meta` prefix cut |
| Per-request long-context split | [`ccusage/rust/adapters/codex/src/aggregate.rs:367-377`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L367-L377) | Port logic | uro-wuby; design §4.5 (context band) | Band decided per request on inclusive input; summary usage rows need a band dimension or request-level pricing |
| ccusage test cases | See [ccusage Tests and Fixtures](#ccusage-tests-and-fixtures) | Adapt tests | uro-obx5; uro-y2qj; uro-spce | Keep ccusage’s input records, write urollup’s expected ledger; add ccusage bugs as negative cases |
| Large-fixture file-size profile | [`ccusage/apps/ccusage/scripts/generate-large-fixture.ts:8-19`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/apps/ccusage/scripts/generate-large-fixture.ts#L8-L19) | Format fact | uro-sbnk | One user’s corpus; do not reuse its unrealistic record generator |
| LiteLLM field list and models.dev catalog trust rules | [`ccusage/rust/crates/ccusage-core/build.rs:154-214`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/build.rs#L154-L214); [`src/models-dev-catalog-rules.json`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/models-dev-catalog-rules.json); [`pricing.rs:276-427`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L276-L427) | Learn; port logic for the maintainer diff script | uro-wuby; design §4.5 | Cross-check only; prefer the authoring provider’s catalog over resellers |
| Fast multipliers, auto-review model dates | [`src/fast-multiplier-overrides.json`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/fast-multiplier-overrides.json); [`ccusage/rust/adapters/codex/src/codex-auto-review-fallbacks.json`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/codex-auto-review-fallbacks.json) | Format fact | uro-wuby | Verify against provider pages before any table row |
| Adapter crate layout and glob re-exports | [`ccusage/rust/adapters/README.md:1-40`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/README.md#L1-L40) | Learn | design §8.1 (two crates) | Supports urollup’s choice of core modules; crate-per-adapter was for Nix build caching |
| Fuzzy model matching, runtime price refresh, `f64` costs, cost-mode mixing | [`pricing.rs:644-680`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L644-L680), [`970-1103`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/pricing.rs#L970-L1103); [`cost.rs:19-192`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/cost.rs#L19-L192) | Avoid | uro-wuby | Excluded by design §4.5; explains cost differences in the feature matrix |
| Two Claude pipelines; kind-dependent Codex keys | [`daily.rs:396-462`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/daily.rs#L396-L462); [`aggregate.rs:531-554`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L531-L554) | Avoid; learn | uro-spce; uro-o65n | One ledger for every report; matrix rows name the ccusage path |
| Statusline session cost and output cache | [`ccusage/rust/crates/ccusage/src/commands/mod.rs:318-787`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/commands/mod.rs#L318-L787) | Learn; avoid | uro-20ck; uro-gnqj | Current-session reads must be scoped to the resolved transcript; cache writes atomic and owner-only |

### ccusage Dialect Facts

These facts correct or extend the brief’s
[dialect survey](research-2026-09-13-portable-agent-usage.md#log-dialects-and-session-linkage).
Facts that rest only on ccusage fixtures, comments or READMEs needed confirmation
against vendor source; the Codex and Pi reviews settled some of them, as noted.

**Claude Code (`claude-project`)**

1. **`iterations` exclude the advisor.** The portable brief first said “top-level counts
   sum server-side `iterations`” (corrected 2026-09-14); in ccusage’s fixture the
   top-level `input_tokens`, `output_tokens`, `cache_creation_input_tokens` and
   `cache_read_input_tokens` equal the sum of the `type: "message"` iterations, and a
   `type: "advisor_message"` iteration carries its own `model` and counts outside the
   top level; the record also has a top-level `advisorModel`
   ([`ccusage/rust/crates/ccusage/src/main.rs:320-343`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L320-L343)).
2. **`progress` records nest subagent assistant messages.** Parent transcripts can hold
   `type: "progress"` records whose `data.message` contains a subagent’s full assistant
   record (`message.id`, `requestId`, `usage`, `isSidechain`, `uuid`, its own
   `timestamp`), duplicating the record in the subagent file
   ([`ccusage/rust/crates/ccusage/src/main.rs:425-484`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L425-L484)).
   The brief’s dialect table lists them, and urollup captures them as observations.
3. **`/btw` side-question logs.** `isSidechain: true` files under `subagents/` (the
   `aside_question` feature) replay parent messages with the same `message.id`, a
   **different `requestId`** and the parent’s cache-read usage
   ([`ccusage/rust/adapters/claude/src/README.md:19-31`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/README.md#L19-L31)).
   A `requestId`-based key alone double counts them.
4. **Gateways reuse message IDs.** Some gateways write the same `message.id` in
   different sessions and omit `requestId`
   ([commit `a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84)).
   This supports the gateway-reuse identity rule in
   [design §3.6](../../urollup-design.md#identity-basis-and-linking).
5. **Usage-limit text.** `isApiErrorMessage: true` records can carry
   `Claude AI usage limit reached|<epoch seconds>`, a reset time
   ([`ccusage/rust/adapters/claude/src/lib.rs:532-557`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L532-L557)).
   It is a provider limit source beside `quotaLimits`.
6. **Other fields:** `usage.speed` takes `standard` or `fast`; `message.model` can be
   `<synthetic>`; top-level `costUSD` appears in older transcripts; the nested legacy
   layout `projects/<project>/<session>/<file>.jsonl` and the root
   `~/.config/claude/projects` both occur
   ([`ccusage/rust/adapters/claude/src/README.md:1-17`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/README.md#L1-L17);
   [`ccusage/rust/crates/ccusage-core/src/types.rs:7-64`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/types.rs#L7-L64)).
7. **`cache_creation` can disagree with `cache_creation_input_tokens`.** ccusage trusts
   the breakdown whenever it exists
   ([`ccusage/rust/crates/ccusage-core/src/types.rs:41-49`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/types.rs#L41-L49));
   urollup keeps both native values and diagnoses a mismatch
   ([design §4.1](../../urollup-design.md#41-measure-contracts)).

**Codex (`codex-rollout`, `codex-exec`)**

8. **Service tier record.** `event_msg` `thread_settings_applied` (CLI 0.144.0 and
   later) carries `thread_settings.service_tier`: `priority` (legacy `fast`) or
   `default`, which Codex Desktop spells `standard` in the same CLI version; auto-review
   threads emit settings events without the key; the event is not written per turn, so
   short rollouts have no tier
   ([`ccusage/rust/adapters/codex/src/README.md:21-28`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/README.md#L21-L28)).
9. **`codex-auto-review` model placeholder.** Auto-review usage records the model name
   `codex-auto-review`, not the serving model
   ([`ccusage/rust/adapters/codex/src/parser.rs:41`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L41),
   [`614-627`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L614-L627)).
   urollup keeps it as observed and leaves it unpriced
   ([design §4.5](../../urollup-design.md#45-price-table)).
10. **Replayed history is re-stamped.** Forked and spawned rollouts rewrite copied
    history to the fork instant and write it in a 10–40 ms burst, with 5.8–15.3 s before
    the child’s first own turn; replays of compacted history do not line up
    event-for-event with the parent
    ([`parser.rs:84-94`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L84-L94);
    [`loader.rs:1693-1723`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1693-L1723)).
    Legacy subagent rollouts start with their own `session_meta`, then the parent’s
    ([`loader.rs:1098-1120`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1098-L1120)).
    Copied history keeps no original timestamps, which the Codex review confirmed:
    copied lines get a new write-time `timestamp` (see
    [Questions Resolved From Source](#questions-resolved-from-source)).
11. **MultiAgent V2 markers (unverified).** ccusage documents `event_msg` `task_started`
    as the end of a subagent’s replayed prefix and `inter_agent_communication_metadata`
    (named `inter_agent_communication` before CLI 0.143.0-alpha.15) with
    `trigger_turn: true` as the start of the child turn; `trigger_turn: false` also
    occurs. Resuming a session re-records the original `cli_version` in `session_meta`,
    so the version cannot gate parsing
    ([`README.md:30-36`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/README.md#L30-L36)).
    No ccusage code uses the markers, and the Codex review did not confirm them (see
    [ccusage Still Unverified](#ccusage-still-unverified)).
12. **`cache_write_input_tokens` may sit inside `input_tokens`**, as ccusage later
    models it
    ([commit `15b3bef`](https://github.com/ccusage/ccusage/commit/15b3bef85b1e0d440ca98e345b4fb5610a41a195)).
    Codex source does not state it (see [Codex Token Usage](#codex-token-usage)), so the
    brief marks it unverified.
13. **Recorded `total_tokens` can be 0** on usable usage objects; derive input plus
    output and record the derivation
    ([`ccusage/rust/adapters/codex/src/types.rs:291-297`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L291-L297)).
14. **OpenAI long context is per request:** above 272K input tokens, cached included,
    the whole request bills at long-context rates
    ([`ccusage/rust/adapters/codex/src/types.rs:69-73`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/types.rs#L69-L73)).
15. **ccusage coverage, not a format fact:** ccusage 20.0.20 reads no `.jsonl.zst`,
    `token_usage_record`, `subagent_history_start_ordinal`, `rate_limits` or
    `turn_context.effort`, and neither does `main` at `95bbc41`. urollup claims no
    parity with ccusage on those.

**Pi (`pi-session`)**

16. **Forks replay the parent’s active branch with original timestamps.** ccusage’s
    later fix matches a child’s leading records against the parent’s root-to-leaf path
    at or before the fork header timestamp, including each record’s timestamp, and links
    through the header `parentSession`
    ([commit `809eeb6`](https://github.com/ccusage/ccusage/commit/809eeb6d52a2c7d13b9c65e10d4106109247390c)).
    Copied entries keep timestamps, and the Pi review confirmed that they also keep
    entry IDs (see [Pi Top Findings](#pi-top-findings)).

### ccusage Bugs and Pitfalls

Each is a ccusage behavior urollup must not reproduce.

1. **Record dropped by a nested null.** `has_unsupported_null_field` scans the raw line
   for `"<name>":null` with `name` in `id`, `cwd`, `model`, `speed`, `costUSD`,
   `version`, `sessionId`, `requestId`, `isApiErrorMessage` and two cache fields, at any
   depth
   ([`ccusage/rust/adapters/claude/src/lib.rs:455-500`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/lib.rs#L455-L500)).
   A synthetic assistant record whose `tool_use.input` was `{"id":null}` was rejected
   (escaped JSON inside strings was not).
   The request then counts through an earlier block record with lower `output_tokens`,
   or not at all.
2. **Order-dependent double count** in Claude `daily` (see
   [ccusage Claude Parsing](#ccusage-claude-parsing)).
3. **Reports that disagree by construction:** Claude `daily` counts progress-embedded
   usage and `monthly` does not; Codex daily and session totals use different keys;
   `--since`, JSON and single-source table runs take different Codex paths (see
   [ccusage Claude Parsing](#ccusage-claude-parsing) and
   [ccusage Codex Parsing](#ccusage-codex-parsing)).
4. **No key, no dedupe:** Claude entries without `message.id` are never deduplicated.
5. **Filter before dedupe:** `--project` changes which copies compete.
6. **Strict timestamps:** only `YYYY-MM-DDTHH:MM:SS`, with optional `.mmm` milliseconds
   and then `Z` or `±HH:MM`, parse
   ([`ccusage/rust/crates/ccusage-core/src/date_utils.rs:111-161`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L111-L161)).
   Other precisions are silently dropped for Claude and Pi, and abort Codex aggregation
   with `Invalid Codex timestamp`
   ([`ccusage/rust/adapters/codex/src/aggregate.rs:251-252`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/aggregate.rs#L251-L252)).
7. **Silent zone fallback:** an invalid `--timezone` becomes the system zone.
8. **Lossy numbers:** negatives, floats and strings in token fields become 0
   ([`ccusage/rust/adapters/common/src/jsonl.rs:64-84`](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/common/src/jsonl.rs#L64-L84));
   cached input is clamped to input; all-zero-component events with a total are dropped.
9. **Replay heuristics that can drop real usage:** a fork whose first two own events
   fall within 1 s, or keep replayed usage: a single-event compressed replay.
   Metadata comes from the first line only.
10. **Path-derived identities:** Codex sessions are file paths; Claude and Pi sessions
    are file names; Claude workflow subagent files become sessions; archived and active
    copies dedupe only by identical relative path.
11. **Mixed-dialect line sniffing:** any rollout line with a top-level `usage` object
    would be counted as `codex exec` usage in addition to `token_count` events.
    urollup should decide the dialect per source and diagnose foreign records.
12. **Fabricated categories:** Pi `totalTokens` remainders become output tokens; missing
    Codex models become `gpt-5`; unrecorded Codex tier follows today’s `config.toml`.
13. **Unknown cost is zero:** unpriced models serialize as cost 0 with no JSON flag.
14. **Pricing inconsistencies:** per-category marginal 200K tiering for LiteLLM tier
    data; `input × 1.25` and `input × 0.1` cache defaults for every provider; cache
    reads at full input rate on the Codex path; 1-hour writes at a hard-coded 2×; one
    total mixing recorded and computed cost.
15. **Non-reproducible prices:** default runs fetch LiteLLM `main` and live models.dev.
16. **`totalTokens` semantics differ** between Claude and Codex JSON, and model
    breakdowns do not sum to rows when models are missing.
17. **Terminal injection risk:** log-supplied names reach the terminal unsanitized.
18. **Current-session cost is a full scan:** statusline rescans every Claude file per
    render, with a shared-temp, non-atomic cache.
19. **Symlinked logs are skipped** without a diagnostic.
20. **README drift:** the Codex README describes replay markers the code does not
    implement; treat ccusage READMEs as claims, not evidence.

### ccusage Still Unverified

- **MultiAgent V2 replay markers** (dialect fact 11): documented in ccusage’s Codex
  README, used by no ccusage code, and not confirmed by the Codex review.
- **`cache_write_input_tokens` inside `input_tokens`** (dialect fact 12): ccusage models
  it that way from commit `15b3bef`, but Codex source does not state it.
- **Service-tier spellings** (dialect fact 8): Codex Desktop’s `standard` and
  auto-review threads without the key come from ccusage’s README only.
- **Replay burst timings** (dialect fact 10): the 10–40 ms bursts and 5.8–15.3 s gaps
  are measurements recorded in a ccusage code comment; the Codex review confirmed the
  re-stamping but not these timings.
- **Claude facts from ccusage fixtures and notes:** advisor iterations, `progress`
  nesting, `/btw` replays and gateway message-ID reuse need confirmation on more Claude
  Code versions.
- **Price facts:** fast-mode multipliers and `codex-auto-review` model dates need
  checking against provider pages before any price table row uses them.
- **Benchmark profile:** the large-fixture file-size profile describes one user’s
  corpus.

## Pi Findings

Pi was reviewed at
[`d981de1`](https://github.com/earendil-works/pi/tree/d981de1229ef899957bbe968bc8dcda02a21f477)
(coding-agent 0.85.1, released 2026-09-05). Pi session files record no Pi version, so
version bounds below come from the changelog.

### Pi Top Findings

1. **Pi fork and clone copy entries with their original entry IDs.** `/fork` and
   `/clone` copy the root-to-leaf path into a new file and keep each entry’s `id` (only
   `parentId` is re-chained around removed labels); `--fork` copies every non-header
   entry of every branch verbatim.
   This resolves a former open question in the portable brief.
2. **Entry IDs are not unique keys across files.** They are the first 8 hex characters
   of a random UUID, collision-checked only within one file, so a bare `id` collides
   across a corpus (about 1% chance at 10,000 entries, about 69% at 100,000). Header
   session IDs can also repeat: `--session-id` accepts caller-chosen IDs, and `/export`
   plus `/import` produce a second file with the same header `id` and no
   `parentSession`.
3. **`pi-session` writes each assistant message once; the multiplicity is across files
   and nested copies.** Usage lives on assistant messages, on `toolResult.usage`
   (tool-run LLM work), and on `compaction.usage` and `branch_summary.usage` (since
   0.81.0). Copies of those usage objects appear in fork, clone, `--fork`, export and
   import files, in compaction `retainedTail`, and in extension `details` (the subagent
   example stores child messages there).
4. **`pi-events` repeats usage in five event types, and the shape changed twice.** Per
   assistant message: `message_start`, each `message_update`, `message_end`
   (authoritative), `turn_end.message` and `agent_end.messages[]`. Before 0.84.0 each
   update carried the cumulative `message`; 0.84.0 removed it, and 0.84.2 added a
   top-level cumulative `usage` to each update.
   metaproc’s `message_final` is a metaproc synthetic record, not a Pi event.
5. **Pi’s own session total is a usable reconciliation check.** `/session` and the
   footer sum every entry in the file (all branches, compacted history included):
   assistant usage, `toolResult.usage`, and compaction and branch-summary usage.
   They do not add `retainedTail` copies, but for a fork file they include the copied
   history.
6. **`usage.cost` is a runtime estimate from Pi’s model catalog.** Rates come from the
   installed catalog or user overrides (custom models default to all-zero rates), with
   tiered input bands, 1h cache writes at 2x input, and an OpenAI service-tier
   adjustment that the record does not store.
   A `cost.total` of 0 does not mean free, and subscription (OAuth) use is not recorded.
7. **An experimental v4 store has explicit usage rows.** `PI_EXPERIMENTAL=1` paths write
   `~/.pi/agent/experimental/sessions` in a transactional JSONL format with
   `kind: "usage"` rows and a synthetic `v3-import` adjustment row holding an aggregate.
   It is not the default in 0.85.1 but is a likely future dialect.
8. **Session files record no Pi version.** The header has `type`, format `version` (1 to
   3), `id`, `timestamp`, `cwd` and optional `parentSession`, so version-bounded caveats
   can only be applied from timestamps or explicit manifests.

### Pi Reuse Table

| Item | File:lines | Reuse mode | Bead or doc section | Notes |
| --- | --- | --- | --- | --- |
| Session header and entry types, including optional `usage` on compaction and branch summaries | [`coding-agent/src/core/session-manager.ts:32-153`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L32-L153) | Format fact; port logic | `uro-qnok`; brief Usage Fields | The Rust `pi-session` record types; no model field on compaction or branch-summary usage |
| v1 to v3 migration (IDs minted on load, `hookMessage` renamed `custom`) | [`session-manager.ts:230-291`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L230-L291), [`session-manager.ts:954-970`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L954-L970) | Port logic | `uro-qnok`, `uro-xm48` | Loading a v1 or v2 file in Pi rewrites it in place with random IDs; urollup must read v1 without inventing stable IDs and treat the rewrite as a replacement |
| Session directory encoding and file name | [`session-manager.ts:472-481`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L472-L481), [`session-manager.ts:926-952`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L926-L952) | Format fact | `uro-qnok`; design §2.1, §2.2 | `--<cwd without leading separator, / \ : replaced by ->--`; file `<ISO timestamp with : and . as ->_<session id>.jsonl`; session ID is UUIDv7 unless `--session-id` |
| Deferred first write and append-only persistence | [`session-manager.ts:1029-1056`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L1029-L1056) | Format fact | `uro-qnok`, `uro-20ck` | No file exists until the first assistant message; then all buffered entries are written with `wx`, and later entries appended with `appendFileSync` |
| Torn-tail repair on load | [`session-manager.ts:513-557`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L513-L557); tests [`test/session-manager/file-operations.test.ts:71-103`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/file-operations.test.ts#L71-L103) | Format fact; port test cases | `uro-qnok`, `uro-obx5`, `uro-gnqj` | Pi appends `\n` after an unterminated final line, so a torn fragment becomes a permanent malformed interior line; malformed lines are skipped silently |
| Fork and clone (`createBranchedSession`) keeping entry IDs | [`session-manager.ts:1422-1544`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L1422-L1544); runtime [`agent-session-runtime.ts:263-355`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session-runtime.ts#L263-L355); test [`test/session-manager/tree-traversal.test.ts:462-516`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/tree-traversal.test.ts#L462-L516) | Format fact; port test cases | `uro-qnok`, `uro-spce`; design §3.2 | Path only; header `parentSession` is the parent’s absolute path; forking before the first user message creates an empty file with `parentSession` and no copied entries |
| `--fork` (`forkFrom`) copying every branch | [`session-manager.ts:1604-1662`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L1604-L1662); [`main.ts:343-345`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/main.ts#L343-L345); test [`test/session-manager/custom-session-id.test.ts:139-168`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/custom-session-id.test.ts#L139-L168) | Format fact | `uro-qnok` | Target cwd may differ from the source’s; all branches, compactions and summary usage are copied |
| JSONL export and import | [`session-export.ts:6-42`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-export.ts#L6-L42); [`agent-session-runtime.ts:361-391`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session-runtime.ts#L361-L391) | Format fact | `uro-qnok`, `uro-spce` | Export keeps the session `id` and entry IDs, writes a new timestamp and no `parentSession`; import copies the file byte for byte into the sessions directory, renaming on name collision |
| Pi’s own session totals and model breakdown | [`agent-session.ts:3326-3381`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L3326-L3381); [`usage-totals.ts:22-70`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/usage-totals.ts#L22-L70); [`footer.ts:86-104`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/modes/interactive/components/footer.ts#L86-L104); tests [`test/agent-session-stats.test.ts:166-258`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/agent-session-stats.test.ts#L166-L258) | Port logic (MIT) as a reconciliation check | `uro-qnok`, `uro-spce`; design §4.1 | Groups model usage by `provider/(responseModel ?? model)` and everything else as “Tools/summaries”; per-file check only, since forks include copies |
| Cache-miss estimator | [`cache-stats.ts:1-164`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/cache-stats.ts#L1-L164); tests [`test/cache-stats.test.ts:60-143`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/cache-stats.test.ts#L60-L143) | Port code (MIT) or port logic, labeled estimate | `uro-d135`, `uro-jpmf` | Better than a fixed “uncached > 100k” threshold; see [agentfdr Heuristics](#agentfdr-heuristics) |
| Cost formula with tiers and 1h writes | [`ai/src/models.ts:891-911`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/models.ts#L891-L911); [`ai/src/types.ts:825-840`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/types.ts#L825-L840); [`coding-agent/docs/models.md:210-213`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/models.md#L210-L213) | Learn only; format fact | `uro-wuby`; design §4.5 | Tier applies to the whole request when `input + cacheRead + cacheWrite` exceeds `inputTokensAbove`, highest matching threshold wins; supports urollup’s context-band rows |
| Per-provider usage normalization | [`anthropic-messages.ts:594-614`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/anthropic-messages.ts#L594-L614), [`anthropic-messages.ts:755-779`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/anthropic-messages.ts#L755-L779); [`openai-responses-shared.ts:556-582`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/openai-responses-shared.ts#L556-L582); [`openai-completions.ts:1517-1546`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/openai-completions.ts#L1517-L1546); [`google-generative-ai.ts:224-240`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/google-generative-ai.ts#L224-L240); [`bedrock-converse-stream.ts:686-696`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/bedrock-converse-stream.ts#L686-L696); [`mistral-conversations.ts:598-606`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/mistral-conversations.ts#L598-L606) | Format fact | `uro-qnok`; brief Usage Fields | See the cache and token semantics table in [Pi Session Dialect Facts](#pi-session-dialect-facts); keyed by the recorded `message.api` |
| Message and usage types (`responseId`, `responseModel`, `providerThinkingLevel`, `diagnostics`, `deferred`, `rawStopReason`, `toolResult.usage`) | [`ai/src/types.ts:17-29`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/types.ts#L17-L29), [`ai/src/types.ts:383-468`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/types.ts#L383-L468) | Format fact | `uro-qnok` | `stopReason` adds `deferred` with a `deferred` handle for batch-style responses |
| Compaction usage combining two calls | [`compaction/compaction.ts:99`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/compaction/compaction.ts#L99), [`compaction.ts:910-961`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/compaction/compaction.ts#L910-L961) | Format fact | `uro-qnok`, `uro-spce` | A split-turn compaction sums the history summary and the turn-prefix summary into one `Usage`, so a compaction entry is one or two requests, not one |
| Assistant persistence on `message_end`, failed attempts kept | [`agent-session.ts:672-691`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L672-L691), [`agent-session.ts:2193-2200`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L2193-L2200), [`agent-session.ts:2918-2921`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L2918-L2921); listeners awaited [`agent/src/agent.ts:588-590`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/agent.ts#L588-L590) | Format fact | `uro-qnok`, `uro-20ck` | Error and truncated attempts stay in the file when auto-retry or overflow recovery removes them from context; extensions can replace the final message in place before it is persisted ([`agent-session.ts:752-756`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L752-L756)) |
| Event emission in the agent loop | [`agent/src/agent-loop.ts:200-272`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/agent-loop.ts#L200-L272), [`agent-loop.ts:312-369`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/agent-loop.ts#L312-L369), [`agent-loop.ts:801-802`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/agent-loop.ts#L801-L802) | Format fact | `uro-qnok` | User, assistant and tool-result messages each get `message_start` and `message_end`; an error or aborted assistant message ends the run with `turn_end` then `agent_end` |
| JSON wire transform and stream header | [`coding-agent/src/modes/json-event.ts:11-61`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/modes/json-event.ts#L11-L61); [`print-mode.ts:109-126`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/modes/print-mode.ts#L109-L126); [`agent-session.ts:144-186`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/agent-session.ts#L144-L186); [`docs/json.md:11-29`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/json.md#L11-L29), [`docs/json.md:86-92`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/json.md#L86-L92) | Format fact; port logic | `uro-qnok`, `uro-obx5` | `pi-events` also carries `agent_end.willRetry`, `auto_retry_*`, `compaction_start/end` (with `result.usage`), `queue_update`, `entry_appended` (extension custom entries only) and `summarization_retry_*` |
| Shell-tool session environment | [`coding-agent/src/core/tools/bash.ts:166-192`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/tools/bash.ts#L166-L192); [`cli/setup.ts:5-7`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/cli/setup.ts#L5-L7); [`docs/environment-variables.md:11-73`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/environment-variables.md#L11-L73) | Format fact | `uro-20ck`; brief Current-Session Signals | See [Pi Current-Session Signals](#pi-current-session-signals) |
| Session directory precedence including `settings.json` | [`coding-agent/docs/settings.md:250-256`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/docs/settings.md#L250-L256) | Format fact | `uro-qnok`; design §2.1, §2.2 | `--session-dir`, then `PI_CODING_AGENT_SESSION_DIR`, then global or project `settings.json` `sessionDir` (relative paths allowed, e.g. `.pi/sessions`); custom directories are flat, with no per-cwd subdirectory |
| Experimental v4 JSONL store | [`agent/src/harness/session/jsonl/types.ts:4-18`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/harness/session/jsonl/types.ts#L4-L18); [`jsonl/codec.ts:35-63`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/harness/session/jsonl/codec.ts#L35-L63); [`jsonl/storage.ts:264-300`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/harness/session/jsonl/storage.ts#L264-L300); [`jsonl/legacy-v3.ts:503-538`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/harness/session/jsonl/legacy-v3.ts#L503-L538); [`session/types.ts:378-395`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/harness/session/types.ts#L378-L395); [`session/fork.ts:70-84`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/agent/src/harness/session/fork.ts#L70-L84); [`coding-agent/src/experimental/server.ts:69-71`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/experimental/server.ts#L69-L71) | Format fact; watch item | Watch item, with no bead yet | Header `{v: 4, kind: "header", id, storageVersion, createdAt, cwd, parentSessionId?, legacyParentSessionPath?, nextSeq?}`; one JSON transaction per line; usage rows `{kind: "usage", id, seq, usage, entryId?, adjustment, details?}`; forks copy entries and values but not usage rows; v3 import re-mints entry IDs and adds one aggregate `adjustment: true` row with `details.source: "v3-import"` |
| Version-bounded changes | [`CHANGELOG.md:237`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L237) (0.84.2), [`:300`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L300) and [`:387`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L387) (0.84.0), [`:569`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L569) (0.82.0), [`:630`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L630) (0.81.0), [`:1336`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L1336) (0.76.0), [`:1642`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L1642) (0.71.0), [`:1839`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L1839) (0.70.0), [`:2130`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L2130) and [`:2140`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L2140) (0.67.1), [`:4597`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/CHANGELOG.md#L4597) (0.31.0) | Format fact | `uro-qnok`, `uro-obx5` | See [Pi Version Bounds](#pi-version-bounds) |
| Legacy v1 session fixtures | [`coding-agent/test/fixtures/before-compaction.jsonl`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/fixtures/before-compaction.jsonl), [`large-session.jsonl`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/fixtures/large-session.jsonl) | Adapt fixtures (after stripping content) | `uro-obx5` | About 1,000 lines each; v1 header with `provider`, `modelId`, `thinkingLevel` and `branchedFrom`; no entry IDs; one lacks `totalTokens`; they hold the maintainer’s real prompts and paths, so keep only structure and usage |
| Migration, load and fork unit cases | [`test/session-manager/migration.test.ts:5-78`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/migration.test.ts#L5-L78); [`test/session-manager/load-entries.test.ts:17-182`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/load-entries.test.ts#L17-L182); [`test/suite/regressions/8989-fork-compaction-label-boundary.test.ts`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/suite/regressions/8989-fork-compaction-label-boundary.test.ts) | Port test cases as synthetic fixtures | `uro-obx5`, `uro-qnok` | Generate the same shapes from Pi’s own writer (MIT) rather than hand-typing records |
| Subagent example (child runs `--mode json -p --no-session`) | [`coding-agent/examples/extensions/subagent/index.ts:148-167`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/examples/extensions/subagent/index.ts#L148-L167), [`index.ts:300`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/examples/extensions/subagent/index.ts#L300), [`index.ts:355-380`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/examples/extensions/subagent/index.ts#L355-L380) | Format fact; learn only | `uro-qnok`; brief Session Linkage | Child sessions are ephemeral, so their only durable usage is the parent’s `toolResult.details.results[].messages[].usage` and an aggregate `details.results[].usage`; neither is the standard `toolResult.usage` |

### Pi Session Dialect Facts

**Identity and location**

- Header: `{type: "session", version?: 1..3, id, timestamp, cwd, parentSession?}`; v1
  has no `version` and no entry IDs, and pre-0.31.0 headers used `branchedFrom` and
  carried `provider`, `modelId` and `thinkingLevel`. No Pi version is recorded anywhere
  in the file.
- Entry base: `{type, id, parentId, timestamp}` with `id` 8 hex characters (full UUID
  only on 100 collisions) and `timestamp` the ISO append time.
  `message.timestamp` is epoch milliseconds set when the provider request object is
  created
  ([`anthropic-messages.ts:520-531`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/ai/src/api/anthropic-messages.ts#L520-L531)),
  so `entry.timestamp - message.timestamp` approximates response duration (inferred).
- Session ID: UUIDv7 since 0.67.1; `--session-id <id>` (0.76.0) accepts any ID of
  letters, digits, `.`, `_` and `-` that starts and ends with a letter or digit
  (`^[A-Za-z0-9]$|^[A-Za-z0-9][A-Za-z0-9._-]*[A-Za-z0-9]$`, equivalent to Pi’s pattern)
  and is unique only within one project directory
  ([`session-manager.ts:212-218`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/session-manager.ts#L212-L218),
  [`cli/args.ts:288`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/cli/args.ts#L288)).
- Entry types beyond the common ones, now in the brief’s dialect table: `custom_message`
  (in context), `label` (`targetId`, `label`) and `session_info` (`name`).
- Default root is `~/.pi/agent/sessions/--<encoded cwd>--/`; a `--session-dir`,
  `PI_CODING_AGENT_SESSION_DIR` or `settings.json` `sessionDir` directory is flat,
  mixing cwds.
- A session file is created only when the first assistant message arrives; a `/fork` at
  the first user message defers its file the same way.
- Files over Node’s maximum string length are supported
  ([`file-operations.test.ts:150`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/test/session-manager/file-operations.test.ts#L150)),
  so sessions can exceed 512 MiB.

**Where usage lives (all four are counted by Pi’s own totals)**

| Carrier | Model attribution | Request count | Since |
| --- | --- | --- | --- |
| `message` with `role: "assistant"`: `message.usage` | `provider`, `model`, `responseModel?`, `api`, `providerThinkingLevel?` | One provider response, including error, aborted and truncated attempts | All versions |
| `message` with `role: "toolResult"`: `message.usage` | None | Unknown: whatever LLM work the tool performed | 0.81.0 |
| `compaction.usage` | None | One or two summary calls combined | 0.81.0 |
| `branch_summary.usage` | None | One summary call | 0.81.0 |

Earlier sessions did not record tool, compaction or branch-summary usage, so their
totals under-report that work; label it as a coverage gap by session timestamp.

**Copies that must not add usage**

| Copy | Created by | Identity preserved | Lineage evidence |
| --- | --- | --- | --- |
| Root-to-leaf path in a new file | `/fork` (position `before`), `/clone` (position `at`), RPC `fork` | Entry `id`, `message` object and `timestamp`; `parentId` re-chained only around dropped labels; label entries get new IDs | Header `parentSession` = absolute path of the source file |
| Every entry of every branch | `pi --fork <path\|id>` | Entry objects verbatim | Header `parentSession` = resolved source path; header `cwd` may differ |
| Active branch with the same header `id` | `/export <file>.jsonl`, then optionally `/import` | Entry IDs and header `id`; new header timestamp | None: no `parentSession` |
| `compaction.retainedTail[]` messages | Harness-generated compactions (per the format doc) | Message objects | Embedded in the same file |
| Extension `details` payloads | e.g. the subagent example’s `details.results[].messages` | Child message objects | Tool result `toolCallId` |
| v1 file after Pi loads it | Pi’s migration rewrite | None: new random IDs on each migration of an unrewritten copy | Replacement of the same path |

**Cache and token semantics by recorded `api`**

| `api` | `input` | `cacheRead` | `cacheWrite` | `cacheWrite1h` | `reasoning` | `totalTokens` |
| --- | --- | --- | --- | --- | --- | --- |
| `anthropic-messages` | `input_tokens` | `cache_read_input_tokens` | `cache_creation_input_tokens` | `ephemeral_1h_input_tokens`, from `message_start` only | `output_tokens_details.thinking_tokens` | Computed sum of the four |
| `openai-responses`, `openai-codex-responses`, `azure-openai-responses` | `input_tokens - cached_tokens - cache_write_tokens` | `input_tokens_details.cached_tokens` | `input_tokens_details.cache_write_tokens` | Absent | `output_tokens_details.reasoning_tokens` | Provider `total_tokens` |
| `openai-completions` | `prompt_tokens - cacheRead - cacheWrite` | `prompt_tokens_details.cached_tokens`, else `prompt_cache_hit_tokens` (DeepSeek), else `cached_tokens` (Kimi) | `prompt_tokens_details.cache_write_tokens` | Absent | `completion_tokens_details.reasoning_tokens` | Computed sum |
| `google-generative-ai`, `google-vertex` | `promptTokenCount - cachedContentTokenCount` | `cachedContentTokenCount` | 0 | Absent | `thoughtsTokenCount`, also added into `output` | Provider `totalTokenCount` |
| `bedrock-converse-stream` | `inputTokens` | `cacheReadInputTokens` | `cacheWriteInputTokens` | Absent | Absent | Provider `totalTokens`, else `input + output` |
| `mistral-conversations` | `prompt_tokens - cached` | Cached prompt tokens | 0 | Absent | Absent | Provider `total_tokens`, else computed sum |

- `input` excludes cache reads and writes, and `reasoning` is a subset of `output`, for
  every adapter above; this confirms the portable brief’s inclusion row.
- `totalTokens` equals the four-field sum only by construction for Anthropic and
  completions APIs; Google, Bedrock and Mistral can differ.
  Recompute totals from components and treat a mismatch as a per-`api` diagnostic, never
  as usage.
- `responseId` is Anthropic `message.id`, the OpenAI Responses `response.id`, the
  completions and Mistral chunk `id`, or Google `responseId`; Bedrock records none.
  `responseModel` is set only by the completions adapter (e.g. OpenRouter `auto`); the
  Anthropic adapter overwrites `model` with the served model instead.
- Before 0.70.0, the completions adapter double-counted reasoning tokens already inside
  `completion_tokens`, so old `openai-completions` output counts can be inflated.

**Cost**

- `usage.cost` is computed at response time from the running install’s catalog or
  `models.json` overrides; custom models default to zero rates, and `pi update --models`
  refreshes catalogs.
- Anthropic OAuth, OpenAI Codex, GitHub Copilot, xAI and Kimi providers can be
  subscriptions
  ([`model-runtime.ts:462-464`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/core/model-runtime.ts#L462-L464));
  the footer marks “(sub)” but the session record keeps the list-price-shaped cost and
  no auth mode
  ([`footer.ts:138-144`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/modes/interactive/components/footer.ts#L138-L144)).
- OpenAI Responses cost can include a service-tier adjustment, and the tier is not
  recorded.
- Treat `cost` as a source-reported estimate
  ([design §3.1](../../urollup-design.md#31-entities)), keep zero distinct from unknown
  only when a catalog rate is known to be nonzero, and never reprice by trusting it.

### Pi Events Dialect Facts

- The first line is the session header, emitted even for `--no-session` runs; an
  in-memory header still has an `id`.
- One assistant message’s usage appears in:

| Event | Usage field | Revision | Version |
| --- | --- | --- | --- |
| `message_start` | `message.usage` | Initial; zeros, or Anthropic input tokens from the provider’s `message_start` | All |
| `message_update` (one per text, thinking or tool-call delta) | `message.usage` inside the cumulative message | Partial | Up to 0.83.x |
| `message_update` | None | None | 0.84.0 to 0.84.1 |
| `message_update` | Top-level `usage` (cumulative), no `message` | Partial | 0.84.2 and later |
| `message_end` | `message.usage` | Final; the persisted object | All |
| `turn_end` | `message.usage`, plus `toolResults[].usage` | Copy | All |
| `agent_end` | `messages[].usage` for every new message in that run | Copy | All |

- Tool results with usage appear in their own `message_start` and `message_end`, then in
  `turn_end.toolResults[]` and `agent_end.messages[]`. `compaction_end.result.usage` is
  a further copy of what the `compaction` entry stores.
- An error or aborted assistant message still gets `message_end`, then `turn_end` and
  `agent_end`. Auto-retry starts another run with its own `agent_end`, so `agent_end`
  arrays do not overlap, but a killed process has no `agent_end` for its last run.
- When the run is persistent, every `message_end` also lands in the session file, so
  `pi-events` and `pi-session` for one run share `responseId`s and message timestamps.
- RPC mode (`--mode rpc`) emits the same session events through the same transform
  ([`modes/rpc/rpc-mode.ts:356`](https://github.com/earendil-works/pi/blob/d981de1229ef899957bbe968bc8dcda02a21f477/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L356)),
  interleaved with RPC responses; a saved RPC transcript is a third captured shape, not
  a supported dialect
  ([design §2.1](../../urollup-design.md#21-dialects-and-discovery)).

### Pi Current-Session Signals

- `PI_SESSION_ID`, `PI_SESSION_FILE`, `PI_PROVIDER`, `PI_MODEL` and `PI_REASONING_LEVEL`
  are set only in LLM-callable `bash` and `powershell` tool processes (0.82.0), resolved
  per command, and never in user-typed `!` or `!!` commands.
- The bash tool deletes inherited `PI_SESSION_*`, `PI_PROVIDER`, `PI_MODEL` and
  `PI_REASONING_LEVEL` before injecting its own, so nested Pi processes never expose a
  stale parent value; it does not remove other agents’ variables.
- `PI_SESSION_FILE` is unset for `--no-session` runs while `PI_SESSION_ID` is still set,
  so an ID without a file means an ephemeral session with no transcript; `--current`
  reports that explicitly rather than searching roots
  ([design §6.2](../../urollup-design.md#62-current-session-detection)).
- The assistant message that called the tool is appended synchronously on `message_end`,
  and the agent loop awaits listeners before running tools, so a Pi current-session
  summary includes the calling request.
  This differs from Claude Code’s asynchronous flush.
- `PI_CODING_AGENT=true` (0.67.1) and `AI_AGENT=pi` (0.84.0) are process markers, not
  session markers; both are inherited.

### Pi Version Bounds

| Version | Change relevant to urollup |
| --- | --- |
| 0.31.0 | Header `branchedFrom` renamed `parentSession` |
| 0.67.1 | UUIDv7 session IDs; `PI_CODING_AGENT=true` |
| 0.70.0 | Completions usage stops double-counting reasoning |
| 0.71.0 | Extensions can replace the finalized `message_end` message before persistence |
| 0.76.0 | `--session-id` custom session IDs |
| 0.81.0 | Usage recorded for tool results, compaction and branch summaries |
| 0.82.0 | `PI_SESSION_ID`, `PI_SESSION_FILE` and related variables |
| 0.84.0 | `message_update` loses the cumulative `message`; `AI_AGENT=pi` |
| 0.84.2 | `message_update` gains top-level cumulative `usage` |
| 0.85.1 | Experimental v4 JSONL and SQLite session backends exist behind `PI_EXPERIMENTAL=1` |

### Pi Dedupe and Fork Rules

The Pi review proposed these rules for `uro-qnok` and `uro-spce`.
[Design §3.4](../../urollup-design.md#34-dialect-reconciliation-rules),
[§3.2](../../urollup-design.md#32-relationships-and-the-discovery-index) and
[Key Scope](../../urollup-design.md#key-scope) adopt the key, lineage and nested-copy
rules, and beads `uro-qnok` and `uro-spce` record the rest.

1. **Request key:** provider namespace from `message.provider` plus `responseId` when
   present. A copy in a fork, clone, export or import file carries the same `responseId`,
   so it reconciles to one request; an auto-retry has a new response and counts
   separately.
2. **Fallback key when `responseId` is absent** (Bedrock, custom APIs, old versions):
   owning-lineage root plus a digest of revision-invariant fields: entry `id`,
   `message.timestamp`, `provider`, `model`, `api`, `stopReason` and the `usage` object.
   Pi re-serializes copies from parsed objects, so these fields are identical across
   fork copies (inferred from the copy code).
   Never key on a bare entry `id`.
3. **Non-message usage** (`toolResult.usage`, `compaction.usage`,
   `branch_summary.usage`): key by lineage root plus entry `id` plus a digest of the
   carrier object; count as calls of unknown count and unknown model, grouped like Pi’s
   “Tools/summaries”.
4. **Lineage:** resolve `parentSession` (and legacy `branchedFrom`) by the source file’s
   basename, `<timestamp>_<session id>.jsonl`, because the stored path is absolute and
   machine-specific. A fork edge makes the parent the owner of every copied entry whose
   key also appears in the parent; an entry found only in the child is child-owned.
   When the parent file is missing, copied entries form a candidate set with
   `unresolved` coverage rather than child-owned usage.
5. **Same header `id` in two files without `parentSession`:** treat as copies of one
   session (export and import); identical keys merge, divergent tails are sibling
   extents of one thread.
   Because custom IDs are project-scoped, include the header `timestamp` and `cwd`
   digest in the `thr-` key alongside the `id`.
6. **Never count** `retainedTail` messages, extension `details` payloads, or any usage
   nested inside another message’s content; record them as evidence only.
7. **`pi-events`:** take `message_end` per message; key as above; within one stream
   without `responseId`, key by `(header id, message.timestamp, provider, model)`. Use
   the last `message_update` usage only when `message_end` is missing, as a partial
   usage revision with a truncated-stream diagnostic; ignore `turn_end` and `agent_end`
   copies except as reconciliation checks.
8. **Check:** per file, the sum of reconciled requests owned by that file plus copies it
   contains should equal Pi’s own `getSessionStats` rule; report differences.

### Pi Pitfalls

- A v1 or v2 file read by urollup before Pi opens it has no entry IDs; after Pi opens
  it, the same path holds different bytes with random IDs.
  urollup’s capture store treats the rewrite as a new source because its first record
  changed, and keeps the old entry as a retained version
  ([design §2.5](../../urollup-design.md#25-capture-store-and-cache)); fallback keys for
  v1 must not depend on minted IDs.
- Pi appends a newline to a torn tail, converting a pending partial line into interior
  corruption; urollup should classify a malformed interior line in `pi-session` as a
  diagnostic, not a fatal error, and should not assume a prefix is immutable only
  because it ends in a newline.
- Pi’s `/session` total double-counts forks relative to their parents; a feature-matrix
  comparison must compare per file, not per corpus.
- `cost.total: 0` is common for providers Pi does not price (metaproc’s `vertex-maas`
  capture); with custom zero-rate models this is indistinguishable from free.
- `compaction` and `branch_summary` usage has no model, so pricing it needs the
  session’s `model_change` context (configured, labeled) or remains unpriced.
- Summing every nested `usage` object in a session line overcounts the subagent example,
  compaction `retainedTail` and `compaction_end` events.
- The v4 `v3-import` adjustment row is an aggregate of the old file; summing it with
  legacy entry usage double-counts, and it is not a request.
- `toolResult.details` can be very large (child transcripts), so the capture policy
  stubs `details` while keeping any `usage` objects inside as evidence
  ([design §2.4](../../urollup-design.md#24-capture-and-export-strip-policies)).

### Pi Still Unverified

- **Runtime behavior:** every Pi fact is source-derived; Pi’s environment in
  `--no-session` runs has not been checked in a running process.
- **Copy invariance** (inferred): that fork copies keep every revision-invariant field
  identical, which the fallback request key relies on, comes from reading the copy code.
- **Response duration** (inferred): `entry.timestamp - message.timestamp` only
  approximates it.
- **Provider totals:** whether `totalTokens` matches the component sum on captures from
  Google, Bedrock and Mistral, which report their own totals.
- **Untested shapes:** saved `--mode rpc` transcripts and the experimental v4 store.
- **Compaction `retainedTail`:** described by the session format doc as
  harness-generated, not traced in code.

## agentfdr Findings

agentfdr was reviewed at
[`e0904bf`](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90)
(0.8.0), a JavaScript tool that normalizes Claude Code and Codex logs into a shared turn
representation.

### agentfdr Top Findings

1. **agentfdr is single-file and has no cross-file dedupe.** It groups Claude block
   records by `message.id` (fallback `uuid`) and overwrites usage, but never
   deduplicates across resumed sessions, forked files or subagent replays, and its usage
   view reads only top-level Claude transcripts, omitting subagent files and Codex.
2. **The `iterations` handling affects totals, not only context size.** agentfdr takes
   context size from the last `iterations` element, and that value also becomes the
   turn’s input and cache counts, which feed session totals, “billed” tokens, cost,
   usage windows and `assert`. When `iterations` has more than one element, agentfdr
   under-reports billed input.
3. **The Codex parser double-counts repeated `token_count` events.** Every `token_count`
   with `last_token_usage` flushes a new turn, with no check that `total_token_usage`
   advanced, so the common repeated snapshot adds usage again; `token_usage_record`,
   forks and subagent replay are not handled.
4. **Its anomaly detectors are the most reusable part.** Loop n-grams with a retry
   allowance and a convergence hint, error streaks, context bloat, token spikes, cache
   thrash, file churn, intent drift, refusals, stalled calls, API errors and user regex
   rules, all configurable and validated.
5. **Subagent tree placement is sound and tested.** `.meta.json` `toolUseId` attaches an
   agent to the spawning turn or to another agent, nested agents inherit the branch’s
   origin turn, agents without a spawn ID are placed by start time, and older inline
   sidechain runs become synthetic nodes.
6. **Adjacent local stores can help account attribution later.** The Board reads Claude
   Desktop’s `claude-code-sessions/<accountUuid>/<orgUuid>/local_*.json` (with
   `cliSessionId`) and `~/.claude.json` `oauthAccount` (`accountUuid`,
   `organizationUuid`, plan tier).

### agentfdr Reuse Table

| Item | File:lines | Reuse mode | Bead or doc section | Notes |
| --- | --- | --- | --- | --- |
| Claude entry-type switch and harness-text filter | [`src/parser.js:28`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L28), [`parser.js:79-123`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L79-L123), [`parser.js:249-295`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L249-L295) | Port logic (MIT) | `uro-y2qj`, `uro-d135` | Types `assistant`, `user`, `system`, `ai-title`, `summary`, `mode`, `permission-mode`, `queue-operation`, `attachment`, `file-history-snapshot`, `last-prompt`; unknown types kept as meta events |
| Queued prompts from `queue-operation` with a 10-minute merge window | [`parser.js:101-116`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L101-L116), [`parser.js:275-288`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L275-L288); tests [`test/parser.test.js:250-309`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/parser.test.js#L250-L309) | Port logic; adapt test cases | `uro-d135` | Prompts typed mid-turn exist only as `enqueue` records; the merge window is a heuristic |
| Compaction collapse (`compact_boundary` plus `summary`) | [`parser.js:317-327`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L317-L327); test [`parser.test.js:286-297`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/parser.test.js#L286-L297) | Port logic | `uro-y2qj` | One compaction writes several records |
| `usage.speed`, `server_tool_use` web search and fetch counts, `stop_reason: "refusal"`, `system` `turn_duration.durationMs` | [`parser.js:179-184`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L179-L184), [`parser.js:297-305`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L297-L305) | Format fact | `uro-y2qj`; brief Usage Fields | `turn_duration` is a native turn timing the brief does not list |
| Model and effort changes parsed from `/model` and `/effort` command stdout | [`parser.js:140-144`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L140-L144), [`parser.js:256-269`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L256-L269); test [`parser.test.js:220-248`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/parser.test.js#L220-L248) | Learn only | `uro-y2qj` | agentfdr claims effort is not a structured field; the brief and squares read entry-level `effort`. Use native `effort` first and treat stdout parsing as an inferred fallback for older versions |
| Codex response-item handling: `exec` JavaScript-snippet commands, block-array outputs, `Wall time`, exit codes, `bash -lc` unwrapping, `apply_patch` file lists | [`src/codex.js:166-233`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js#L166-L233), [`codex.js:262-348`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js#L262-L348); test [`test/codex.test.js:96-132`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/codex.test.js#L96-L132) | Port logic (MIT); adapt fixtures | `uro-y2qj` (tools), `uro-obx5` | Shapes “observed in actual codex-cli 0.14x rollouts”; harness user-message tags at [`codex.js:17`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js#L17) |
| Codex `session_meta` nesting under `payload.meta` and `git.branch` placement | [`codex.js:114-126`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js#L114-L126) | Format fact (unverified) | `uro-y2qj` | Suggests some rollouts nest identity under `payload.meta`; check against Codex source before relying on it |
| Subagent file discovery including `workflows/wf_*` | [`src/subagents.js:3-58`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js#L3-L58) | Port logic | `uro-20ck` | Walks up to depth 6; only `wf_*` directory names are workflow IDs; `meta.agentId` can override the file-name ID |
| Spawn attribution and nesting | [`subagents.js:105-178`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js#L105-L178); tests [`test/subagents.test.js:21-127`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/subagents.test.js#L21-L127) | Port logic; adapt test cases | `uro-20ck`; design §3.2 | Spawn tool names `Task`, `Agent` and `Workflow` ([`subagents.js:180`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js#L180)); time placement is a labeled fallback, never an ownership edge |
| Inline sidechain runs as synthetic nodes | [`subagents.js:188-229`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js#L188-L229) | Port logic | `uro-20ck` | Matches the brief’s inline-sidechain relationship; attributes a run to the preceding spawn call |
| Anomaly detectors | [`src/detect.js:11-581`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/detect.js#L11-L581); tests [`test/detect.test.js:71-252`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/detect.test.js#L71-L252), [`test/parser.test.js:100-191`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/parser.test.js#L100-L191) | Port logic (MIT), labeled estimates | `uro-jpmf` (`check`), Phase 2 report | See [agentfdr Heuristics](#agentfdr-heuristics) |
| Detector config with strict validation | [`src/config.js:1-109`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/config.js#L1-L109); tests [`test/config.test.js`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/config.test.js) | Port logic | `uro-jpmf` | Unknown thresholds, detectors or bad regexes are hard errors: “a CI gate running with half its rules dropped must not pretend it’s configured” |
| `assert` CI gate | [`src/assert.js:22-53`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/assert.js#L22-L53) | Learn only | `uro-jpmf` (`check`) | Fails on fully unpriced cost, but a partially priced session passes `--max-cost`; urollup’s `--require-priced` is stricter and should stay so |
| Claude Desktop session store and CLI account | [`src/board.js:1-27`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/board.js#L1-L27), [`board.js:243-355`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/board.js#L243-L355); [`src/usage.js:24-35`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/usage.js#L24-L35) | Format fact (unverified against Anthropic docs) | `uro-um7n` | Desktop store values are all strings; `~/.claude.json` names only the current login and holds credential-adjacent data, so read only named keys and never by default |
| Loopback Host and same-origin checks | [`src/server.js:117-124`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/server.js#L117-L124), [`server.js:289-308`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/server.js#L289-L308) | Learn only | `uro-tzjq` | Host allowlist and `Sec-Fetch-Site` handling agree with design §7.3; agentfdr has no token on read routes, which urollup adds |
| Viewer data model (`session`, `turns[]` with `usage`, `contextTokens`, `toolCalls[].result`, `prompts[]` with `afterTurn`, `metaEvents[]`, `flags[]`) | [`parser.js:9-18`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L9-L18); [`server.js:198-237`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/server.js#L198-L237) | Learn only | `uro-tzjq` (optional timeline) | Good timeline shape; strips tool inputs from the wire; its turn is a message, not a reconciled request |
| Cost table | [`src/cost.js:10-83`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/cost.js#L10-L83); tests [`test/cost.test.js:5-46`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/cost.test.js#L5-L46) | Learn only | `uro-wuby`, `uro-o65n` | Useful for the feature matrix: explains why agentfdr and urollup costs differ |
| Synthetic in-test fixtures (Claude lines, Codex rollout, subagent directories) | [`test/parser.test.js:9-46`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/parser.test.js#L9-L46); [`test/codex.test.js:11-46`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/codex.test.js#L11-L46); [`test/subagents.test.js:21-38`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/subagents.test.js#L21-L38) | Adapt fixtures (MIT) | `uro-obx5` | Fully synthetic, no real content; add usage variation and repeated `token_count` cases before reuse |

### agentfdr Dialect Facts

- Claude block records sharing `message.id` each carry the full usage object; agentfdr
  overwrites
  ([`parser.js:185-200`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L185-L200)),
  so it effectively takes the last block’s usage.
- Top-level Claude `input_tokens` and cache counts sum server-side `iterations`; the
  last iteration reflects the context the model saw, and output stays top-level because
  every iteration’s output was paid for (agentfdr’s stated rule, same lines).
- `session.id` comes from the first record’s `sessionId`
  ([`parser.js:73-76`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/parser.js#L73-L76)),
  which is wrong for resumed or forked files whose copied records carry another
  session’s ID; the brief’s file-stem rule is correct.
- Codex rollouts: `input_tokens` includes cached tokens, and agentfdr clamps
  `input - cached` at zero “in case some build ships exclusive semantics”
  ([`codex.js:79-91`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/codex.js#L79-L91)),
  consistent with the brief.
- `developer`-role Codex messages and user messages starting with `environment_context`,
  `user_instructions`, `ENVIRONMENT`, `turn_aborted`, `permissions` or `AGENTS` are
  harness text.
- Zero-usage pseudo-model `<synthetic>` appears on Claude records
  ([`cost.js:69-73`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/cost.js#L69-L73)).
- Claude Code live registry `~/.claude/sessions/<pid>.json` also carries `name`,
  `entrypoint` (`cli` or `claude-desktop`), `kind` (`interactive` or `bg`), `status`,
  `waitingFor` and a tmux pane; live transcripts “reach 100 MB”
  ([`board.js:1-27`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/board.js#L1-L27)).
  A second Claude account lives in its own `CLAUDE_CONFIG_DIR` with its own
  `.claude.json`
  ([`board.js:250-255`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/board.js#L250-L255)).

### agentfdr Heuristics

**Worth offering as labeled estimates** (Phase 2 `check` rules or report findings, never
totals):

| Heuristic | Definition in agentfdr | urollup adaptation |
| --- | --- | --- |
| Tool loop | Same n-gram (n = 1 to 4) of `tool:target` signatures repeated at least 3 times over at least 6 calls; edit and verify alternation or test, build, lint and install idioms need 6 repeats; config suppressions; convergence hint from result shapes ([`detect.js:52-180`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/detect.js#L52-L180)) | Port with the retry allowance and the “never all-clear” convergence hint; compute over tool actions from the ledger |
| Error streak | At least 3 consecutive failing tool results | Port as is |
| Context bloat | One tool result of at least 50,000 characters | Use recorded result bytes, labeled as bytes, not tokens |
| Token spike | Context jump over 50,000 tokens and over 1.6x the previous main-thread turn | Use reconciled request input size (uncached plus cache reads plus writes), per thread |
| Stalled call | Tool calls with no result in a turn the session moved past; last turn exempt | Port; exempt the snapshot tail |
| Refusal and API error | `stop_reason: "refusal"`; strict provider-error regex only on results marked as errors | Port; prefer native error records |
| Custom regex rules | User rules over tool results or assistant text, validated at load | Defer: requires content that the capture and export policies stub (design §2.4) |

**Avoid or replace:**

- **Cache thrash** (at least 2 consecutive turns with zero cache reads and more than
  20,000 new input tokens after the third turn) misfires on providers that never report
  caching and after compaction; Pi’s cache-miss estimator handles both (see
  [Pi Reuse Table](#pi-reuse-table)).
- **Intent drift** compares edited paths to prompt words and needs prompt text, which
  exports omit; keep it out of the accounting CLI.
- **File churn** (same file edited 6 or more times) needs tool arguments; offer only
  with detailed evidence enabled.
- **Reconstructed 5-hour windows**
  ([`usage.js:128-151`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/usage.js#L128-L151))
  duplicate the ccusage `blocks` estimate that
  [design §4.4](../../urollup-design.md#44-usage-windows) excludes.
- **“Billed” tokens** as input plus cache writes plus output, dropping cache reads, is a
  private metric; urollup should show token categories separately and price them.

### agentfdr Pitfalls

- Pricing is regex first-match over model names with a 1.25x cache-write multiplier for
  every model, so 1h cache writes are underpriced, `gpt-5` prices any `gpt-5*` model
  (including mini and codex variants), long-context bands are ignored, and money is
  floating point
  ([`cost.js:10-52`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/cost.js#L10-L52)).
- Usage aggregation reads only `<project>/<id>.jsonl` main files
  ([`usage.js:45-61`](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/usage.js#L45-L61)),
  so subagent usage is missing, while resumed copies are counted twice.
- Subagent tree totals parse each agent file independently; a fork-style subagent that
  replays parent records (see
  [Plugin Dialect Facts and Corrections](#plugin-dialect-facts-and-corrections)) adds
  the parent’s usage to the subagent’s summary (inferred).
- Parse errors count every malformed line, including a pending tail, and files are read
  whole into memory.
- `resolveSession()` with no reference picks the newest file by modification time, the
  implicit heuristic that urollup’s `--latest` never runs by default
  ([design §6.2](../../urollup-design.md#62-current-session-detection)).
- Codex discovery skips `archived_sessions/`, `.jsonl.zst` and nested thread linkage.

### agentfdr Still Unverified

- **Codex `payload.meta` nesting:** agentfdr reads identity under `payload.meta`; the
  Codex review found `git` as a sibling of the session metadata fields at
  `rust-v0.154.0`, and older builds were not checked.
- **Claude Desktop and CLI account stores:** the `claude-code-sessions/` store and
  `~/.claude.json` `oauthAccount` fields are unverified against Anthropic documentation.
- **Entry-level `effort`:** agentfdr claims effort appears only in command output, while
  the brief and squares read an entry-level field.
- **Fork-style subagent totals** (inferred): replayed parent records add the parent’s
  usage to a subagent’s summary.
- **Live registry fields:** `name`, `entrypoint`, `kind`, `waitingFor` and the tmux pane
  in `~/.claude/sessions/<pid>.json` come from agentfdr only.

## Anthropic session-report and receipts Findings

The plugins were reviewed at
[`f0dce59`](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537):
session-report in full, and receipts as an adjacent Anthropic usage miner with stronger
dedupe notes and skill rules.
Their transcript notes are empirical observations, not a format specification.

### Plugin Top Findings

1. **The dedupe rule is subtler than “by request ID and by uuid”.** The request key is
   `requestId`, else `message.id` only when it starts with `msg_0` and is longer than 10
   characters, else `<file path>:<uuid>`. Within one file the record with the largest
   `output_tokens` wins (ties go to the later record); across files the first file
   processed wins, with no maximum.
2. **Replay dedupe is global first-seen by `uuid`, and order decides attribution.** Main
   transcripts are processed before subagent files, so fork-style subagent replays of
   parent records go to the parent; but main files are processed in directory-walk
   order, so a resumed session’s replayed records are attributed to whichever main file
   is walked first.
3. **Two new Claude facts:** fork-style subagents replay parent entries with identical
   `uuid`s, and the sibling receipts miner reports that about 13% of block records of
   one response disagree on `output_tokens` while streaming.
   Together they resolve part of the brief’s unverified item on differing usage within
   one request group.
4. **Attribution is heuristic and clearly scoped, which makes it a good estimate
   model.** Prompt attribution follows human prompts until the next one (excluding task
   notifications, scheduled wake-ups and background tasks), subagent usage inherits the
   spawning prompt through `toolUseResult.agentId`, skill attribution runs from
   invocation to the next plain human message, and each session’s tokens go to the local
   day of its first in-range record.
5. **The skill model is “deterministic JSON plus a fixed template plus a short agent
   narrative”.** receipts adds the rules urollup’s skill needs: project names are data,
   say which columns add up, unknown is not zero, no dollar figures from estimates, and
   nothing is published by default.

### Plugin Reuse Table

| Item | File:lines | Reuse mode | Bead or doc section | Notes |
| --- | --- | --- | --- | --- |
| Empirical transcript notes | [`session-report/skills/session-report/analyze-sessions.mjs:14-28`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L14-L28) | Format fact | brief Synthetic Double-Counting Example | Cite as empirical, not a specification |
| Request key and per-file max-output selection | [`analyze-sessions.mjs:304-321`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L304-L321), [`analyze-sessions.mjs:348-351`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L348-L351) | Learn only; format fact | `uro-y2qj`, `uro-spce` | See [Plugin Dialect Facts and Corrections](#plugin-dialect-facts-and-corrections) |
| Global `uuid` dedupe and file ordering | [`analyze-sessions.mjs:161-162`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L161-L162), [`analyze-sessions.mjs:226-230`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L226-L230), [`analyze-sessions.mjs:541-554`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L541-L554) | Format fact; learn only | `uro-y2qj`, `uro-spce` | Replay ownership should come from lineage evidence, not processing order |
| Subagent classification and type resolution | [`analyze-sessions.mjs:111-156`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L111-L156), [`analyze-sessions.mjs:247-265`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L247-L265), [`analyze-sessions.mjs:284-302`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L284-L302), [`analyze-sessions.mjs:560-570`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L560-L570) | Port logic (Apache-2.0 notice if code is ported); format fact | `uro-20ck`; purpose (design §3.5) | Order: `.meta.json` `agentType`, else file-name label, else the parent’s `Agent`/`Task` `input.subagent_type` linked through `toolUseResult.agentId`, else `fork`; this gives an observed purpose for Phase 1 |
| Agent ID shape and internal fork labels | [`analyze-sessions.mjs:151-156`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L151-L156), [`analyze-sessions.mjs:796-798`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L796-L798) | Format fact (cites Claude Code internal `src/utils/uuid.ts`) | `uro-20ck`, `uro-52qi` | `agent-a<16 hex>` or `agent-a<label>-<hex>`; labeled files are internal background forks such as `task_summary` and `compact`, not user subagents |
| Prompt and skill attribution | [`analyze-sessions.mjs:420-480`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L420-L480), [`analyze-sessions.mjs:368-377`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L368-L377) | Port logic, labeled estimate | `uro-52qi`, `uro-jpmf` | Human prompt = main file, not sidechain, not `isMeta` or `isCompactSummary`, string or first text block, not a tool result or `[Request interrupted`; slash commands from `<command-name>` or `<command-message>` |
| Cache breaks with prompt context | [`analyze-sessions.mjs:386-404`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L386-L404), [`analyze-sessions.mjs:494-512`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L494-L512) | Learn only | `uro-d135` | `input_tokens + cache_creation_input_tokens > 100,000` (flag `--cache-break`); replace with the Pi estimator’s previous-prompt comparison |
| Day timeline and peak concurrency | [`analyze-sessions.mjs:678-720`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L678-L720) | Learn only | `uro-tzjq` | Whole session on its first local day, 10-minute buckets, capped at 24 hours; conflicts with urollup’s clipped time filters |
| Output JSON shape (`overall`, `by_project`, `by_subagent_type`, `by_skill`, `cache_breaks`, `top_prompts`, `by_day`) | [`analyze-sessions.mjs:616-676`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L616-L676) | Learn only | `uro-jpmf`, `uro-o65n` | A feature-matrix row: which of these urollup reproduces exactly, and which it labels estimates |
| Skill steps | [`session-report/skills/session-report/SKILL.md:1-42`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/SKILL.md#L1-L42) | Learn only (structure) | `uro-jpmf` | See [Reporting Skill Model](#reporting-skill-model) |
| Template with data slot and two agent slots | [`template.html:245-251`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/template.html#L245-L251), [`template.html:315-321`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/template.html#L315-L321), [`template.html:336-352`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/template.html#L336-L352) | Learn only | `uro-jpmf` | Rendering stays deterministic; the agent only fills findings and suggestions |
| receipts: field-wise max usage merge and its 13% observation | [`receipts/skills/receipts/scripts/mine-transcripts.mjs:412-432`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L412-L432), [`mine-transcripts.mjs:495-501`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L495-L501), [`mine-transcripts.mjs:578-586`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L578-L586) | Format fact; learn only | `uro-y2qj`; brief Next Steps | Field-wise max can produce a combination no record held (inferred), so urollup should select one record, not merge fields |
| receipts: 1h and 5m cache-write split and relative weights | [`mine-transcripts.mjs:170-212`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L170-L212) | Format fact; learn only | `uro-wuby` | “Near an even split” on a real corpus, so pricing flattened `cache_creation_input_tokens` as 5m writes misprices heavily |
| receipts: `promptId` prompt counting | [`mine-transcripts.mjs:556-576`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L556-L576) | Format fact | `uro-y2qj`, `uro-d135` | User entries carry `promptId`, a native prompt key now in the brief’s dialect table (agentfdr also reads `entry.promptId`) |
| receipts: project attribution by touched paths | [`mine-transcripts.mjs:111-130`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L111-L130), [`mine-transcripts.mjs:638-720`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L638-L720) | Port logic, labeled estimate (later) | `uro-52qi` | Session spend split by file-touch votes, cwd fallback only inside a git repo, else a no-project bucket; filters applied after attribution |
| receipts: skill rules | [`receipts/skills/receipts/SKILL.md:37-48`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L37-L48), [`SKILL.md:139-164`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L139-L164), [`SKILL.md:217`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L217), [`SKILL.md:228-261`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L228-L261), [`SKILL.md:268-290`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L268-L290) | Learn only (adapt wording) | `uro-jpmf` | See [Reporting Skill Model](#reporting-skill-model) |

### Plugin Dialect Facts and Corrections

- **Correction to the portable brief’s earlier “deduplicates by request ID and by
  `uuid`” (corrected 2026-09-14):** the key falls back to `message.id` only for
  `msg_0`-prefixed IDs longer than 10 characters, else to a per-file, per-record key;
  the maximum-`output_tokens` rule is per file only
  ([`analyze-sessions.mjs:197-199`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L197-L199)),
  and first-processed wins across files.
  Records without `requestId` whose message IDs have another prefix (inferred: gateway
  or cloud-provider message IDs) are counted once per block record.
- **Extension:** fork-style subagent transcripts begin by replaying parent entries with
  identical `uuid`s
  ([`analyze-sessions.mjs:541-544`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L541-L544)).
  Combined with ccusage’s sidechain replay under new request IDs, `uuid` equality is
  lineage evidence that a subagent record is a parent copy, even when `requestId`
  differs.
- **Extension:** subagent files without `.meta.json` and without a label default to type
  `fork`; a path containing `workflows` but not `subagents` is treated as
  `<project>/<session>/workflows/…`
  ([`analyze-sessions.mjs:132-135`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L132-L135)),
  a layout agentfdr does not list and no inspected local log contained (unverified).
- **Confirmed:** “wall clock” sums each file’s first-to-last span, including subagent
  files that overlap the parent, and “active” sums in-file gaps under 5 minutes
  ([`analyze-sessions.mjs:232-245`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L232-L245),
  [`analyze-sessions.mjs:326-335`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L326-L335)).
- **Project** is the encoded directory name (`parts[0]`), not recorded `cwd`
  ([`analyze-sessions.mjs:115-117`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L115-L117)).
- **Auto-continuation markers** that are not human prompts: user text starting with
  `<task-notification`, `<scheduled-wakeup` or `<background-task`
  ([`analyze-sessions.mjs:442-450`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L442-L450));
  agentfdr adds `<system-reminder`, `<local-command` and command tags.
  receipts, by contrast, counts scheduled or queued invocations as prompts because “the
  dev set it up”.
- **receipts** deduplicates responses per file only and globally by `uuid`, skips files
  whose modification time predates the window, and drops records before the window
  before the `uuid` check
  ([`mine-transcripts.mjs:486`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L486),
  [`mine-transcripts.mjs:535-543`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L535-L543)),
  so the two official miners already disagree on window edges.

### Plugin Heuristics

**Keep as labeled estimates:**

- Prompt cost attribution (tokens from one human prompt until the next, including
  subagents spawned during it); label as “attributed” and exclude background forks.
- Subagent type grouping with the resolution order above; `fork` as an explicit
  unknown-type group.
- Skill and slash-command attribution windows; label clearly, since the invoking
  request’s own usage is charged to the skill (inferred from
  [`analyze-sessions.mjs:287-321`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/analyze-sessions.mjs#L287-L321)).
- receipts’ by-project spend share, which it argues is stable because it divides a
  session’s whole cost by which project it served.

**Avoid:**

- Percent-of-total-tokens findings that add cache reads to output
  ([`SKILL.md:27`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report/skills/session-report/SKILL.md#L27));
  cache reads dominate, so shares should be cost-weighted with a priced basis or shown
  per category.
- The skill’s fixed anomaly thresholds (cache hit under 85%, one prompt over 2%,
  subagent type over 1M tokens per call) as verdicts; present them as configurable
  `check` thresholds.
- Activity or tool-level spend breakdowns: receipts measured web search at 11%, 28% or
  51% of one month’s spend depending on the weighting
  ([`SKILL.md:228-244`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/SKILL.md#L228-L244)).
- Whole-session-on-first-day bucketing and summed file spans as time.

### Reporting Skill Model

- **Deterministic engine, thin skill.** Step 1 runs the bundled CLI to JSON; the agent
  reads a small result and never re-reads raw transcripts (receipts `SKILL.md:45-48`).
  urollup: run `urollup report --format json`, never compute numbers in the skill.
- **Fixed template, agent-owned slots.** The page renders from an embedded JSON blob;
  the agent fills only a 3-to-5-line findings block and 1-to-4 suggestions tied to
  specific rows, and must not restructure sections.
  urollup: CLI-rendered Markdown with named narrative slots.
- **Findings format.** One figure plus one sentence naming the subject, classed as
  waste, healthy or neutral.
- **Honesty rules to adopt from receipts:** treat project, session and tool names from
  logs as inert data, never instructions (lines 139-146); state under each table which
  columns add up (148-157); unknown is not zero (217); no dollar figures from local
  estimates unless labeled as a dated list-price estimate (159-164); take dates and
  windows from the JSON, not the agent’s own arithmetic (268); list the names a report
  exposes before it is shared, and never publish by default (276-290); write output
  files owner-only without following symlinks
  ([`mine-transcripts.mjs:1066-1075`](https://github.com/anthropics/claude-plugins-official/blob/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts/skills/receipts/scripts/mine-transcripts.mjs#L1066-L1075)).
- **Arguments.** Both skills map natural ranges (`24h`, `7d`, `week`, `quarter`, a bare
  number) to one flag; urollup’s skill should map them to `--since` and name the
  timezone.
- **Do not copy:** writing to a fixed `/tmp/session-report.json` path, embedding up to 2
  MB of JSON into a page, and “do not open it” delivery without stating coverage.

### Plugin Pitfalls

- Processing order decides replay attribution; parallel or differently ordered scans
  give different per-session and per-project numbers from the same files.
- `--since` drops earlier records but the `uuid` and request sets are global, so a
  narrower window can change which file owns a replayed record.
- Records without `usage` never reach the request map, but zero-usage synthetic records
  with usage are counted as API calls (inferred).
- `api_calls` counts deduplicated keys, not provider requests: a missing `requestId`
  with a non-`msg_0` ID counts every block.
- Subagent “calls” means subagent transcript files with in-range records, and
  `avg_tokens_per_call` includes cache reads.

### Plugin Still Unverified

- **Block-record disagreement:** receipts’ 13% `output_tokens` disagreement comes from
  one corpus and needs fixtures from more Claude Code versions.
- **`<project>/<session>/workflows/` layout:** session-report handles it, but no other
  reviewed source or inspected log shows it.
- **Other message-ID prefixes** (inferred): records without `requestId` whose message
  IDs lack the `msg_0` prefix are likely gateway or cloud-provider IDs.
- **Zero-usage synthetic records** (inferred): counted as API calls.
- **Field-wise maximum** (inferred): receipts’ merge can produce a combination no record
  held.
- **Skill attribution** (inferred): the invoking request’s own usage is charged to the
  skill.
- **Agent ID shape:** session-report cites Claude Code’s internal `src/utils/uuid.ts`,
  which is not public.

## Comparison Matrix

How the reviewed tools and the urollup design resolve the same records:

| Behavior | ccusage 20.0.20 | agentfdr 0.8.0 | session-report | receipts | urollup design |
| --- | --- | --- | --- | --- | --- |
| Claude request key | `(message.id, requestId)`; entries without `message.id` are never deduplicated | `message.id`, else `uuid`, within one file | `requestId`, else a `msg_0` `message.id` longer than 10 characters, else file path and `uuid` | Responses per file, plus a global `uuid` set | Provider, `message.id` and `requestId` ([§3.4](../../urollup-design.md#34-dialect-reconciliation-rules)) |
| Disagreeing block records | Non-sidechain copy, then largest four-field total, then `speed`; the whole record wins | The last block’s usage overwrites earlier ones | Largest `output_tokens` within a file, ties to the later record | Field-wise maximum | One whole record: largest `output_tokens`, then last in file, then lowest `src-` ID, with a diagnostic |
| Copies across files | Sidechain fallback on `message.id`; session-scoped from commit `a4b8420` | None | Global first-seen `uuid`, so processing order decides attribution | Global `uuid`, after dropping records before the window | Lineage evidence: `progress`, `/btw` and same-`uuid` records are parent-owned copies |
| Codex `token_count` events | `last_token_usage` when the total advances; replays skipped by value match or a 1 s burst | Every event with `last_token_usage` adds usage | Claude Code transcripts only | Claude Code transcripts only | `token_usage_record` by `response_id`; otherwise counter epochs and child boundaries |

## Recommendations Status

Each review ended with recommendations for the portable brief, the plan, the data
contracts and the beads.
This section maps each one to where it was applied, checked on 2026-09-15 against the
[urollup design](../../urollup-design.md), the
[plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md), the
[portable brief](research-2026-09-13-portable-agent-usage.md) and bead notes.

- **Applied:** reflected where the review aimed it.
- **Partial:** partly reflected; the missing part is named.
- **Queued:** a pending maintainer decision in
  [design §9.2](../../urollup-design.md#92-queued-review-decisions), walked through in
  bead `uro-gxen`.
- **Open:** not reflected in any doc or bead.

| Review | Recommendations | Applied | Partial | Queued | Open |
| --- | ---: | ---: | ---: | ---: | ---: |
| Codex | 20 | 16 | 3 | 1 | 0 |
| ccusage | 18 | 12 | 6 | 0 | 0 |
| Pi, agentfdr and Anthropic plugins | 23 | 18 | 3 | 0 | 2 |
| **Total** | **61** | **46** | **12** | **1** | **2** |

### Codex Recommendations

| # | Recommendation | Status | Where applied |
| ---: | --- | --- | --- |
| 1 | Describe `turn.completed.usage` as the thread’s cumulative total, with no `total_tokens`, primary thread only, and no usage for failed or interrupted turns | Applied | Brief [Usage Fields](research-2026-09-13-portable-agent-usage.md#usage-fields); [design §3.4](../../urollup-design.md#34-dialect-reconciliation-rules) |
| 2 | Record `token_usage_record`’s first release, `rust-v0.153.0`, and that subagent tools get their own `CODEX_THREAD_ID` | Applied | Brief [Usage Fields](research-2026-09-13-portable-agent-usage.md#usage-fields) and [Current-Session Signals](research-2026-09-13-portable-agent-usage.md#current-session-signals); [design §6.2](../../urollup-design.md#62-current-session-detection) |
| 3 | Correct Codex hook input: `session_id` is the root session, and hooks get no `CODEX_*` variables | Applied | Brief [Current-Session Signals](research-2026-09-13-portable-agent-usage.md#current-session-signals); [design §6.2](../../urollup-design.md#62-current-session-detection) |
| 4 | Add the `inter_agent_communication*`, `world_state`, `retained_context`, `security_risk_score` and `realtime_item` item types and the `response_item` `metadata` field | Partial | Item types in the brief’s [dialect table](research-2026-09-13-portable-agent-usage.md#log-dialects-and-session-linkage); `inter_agent_communication_metadata` and the `metadata` field only in [Rollout Item Types](#rollout-item-types) |
| 5 | Note that `rate_limits` carries `limit_id`, `credits`, `plan_type`, `individual_limit` and `spend_control_reached` | Applied | Brief [Usage Fields](research-2026-09-13-portable-agent-usage.md#usage-fields); [design §3.1](../../urollup-design.md#31-entities) |
| 6 | Note local-time file names, flat `archived_sessions/`, feature-gated compression and migration, and the fixed SQLite file names | Partial | Brief [Log Dialects and Session Linkage](research-2026-09-13-portable-agent-usage.md#log-dialects-and-session-linkage); [design §2.1](../../urollup-design.md#21-dialects-and-discovery); the SQLite file names only in [Codex Session Identity](#codex-session-identity) |
| 7 | Check `--hook-input` by `session_meta.session_id` and `agent_id`, take `agent_transcript_path` on `SubagentStop`, and never rely on environment variables inside Codex hooks | Applied | [Design §6.2](../../urollup-design.md#62-current-session-detection); `uro-20ck` |
| 8 | Use SQLite `threads.rollout_path` and `thread_spawn_edges` as an optional, version-gated discovery hint, never a usage source | Partial | `uro-20ck` and brief [Current-Session Signals](research-2026-09-13-portable-agent-usage.md#current-session-signals); [design §2.1](../../urollup-design.md#21-dialects-and-discovery) and [§3.2](../../urollup-design.md#32-relationships-and-the-discovery-index) do not list the hint |
| 9 | Treat a `.jsonl` rollout and its `.jsonl.zst` twin with one thread and rollout ID as one source whose representation changed | Applied | [Design §2.2](../../urollup-design.md#22-snapshot-boundary); `uro-xm48`, `uro-gnqj` |
| 10 | Key requests by provider and `token_usage_record.response_id`, owned by the record’s `thread_id`; treat foreign-thread records as copies and never observe `compacted.latest_token_usage_record` | Applied | [Design §3.4](../../urollup-design.md#34-dialect-reconciliation-rules) and [§3.6](../../urollup-design.md#36-analytical-identities) |
| 11 | Apply a counter-epoch rule to pre-0.153 files and exclude records before the child boundary | Applied | [Design §3.3](../../urollup-design.md#33-reconciliation) and [§3.4](../../urollup-design.md#34-dialect-reconciliation-rules); `uro-y2qj` |
| 12 | Link `codex-exec` captures to rollouts by `thread.started.thread_id`, with turn deltas from consecutive totals | Applied | [Design §2.6](../../urollup-design.md#26-harness-captures) and [§3.4](../../urollup-design.md#34-dialect-reconciliation-rules) |
| 13 | Key Codex `src-` IDs by thread and rollout ID plus the first record digest, so archive and compression keep the ID | Applied | [Design §2.1](../../urollup-design.md#21-dialects-and-discovery) and [§3.6](../../urollup-design.md#36-analytical-identities); plan [Testing Strategy](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#testing-strategy) |
| 14 | Add `limit_id`, `limit_name`, `plan_type`, `credits`, `individual_limit`, `spend_control_reached` and `rate_limit_reached_type` to provider limit observations, deduplicate identical snapshots, and mark carried-forward fields as possibly stale | Applied | [Design §3.1](../../urollup-design.md#31-entities), which keeps native limit fields verbatim; `uro-eamm` |
| 15 | Label Codex request models `requested` | Applied | [Design §3.1](../../urollup-design.md#31-entities) and [§4.5](../../urollup-design.md#45-price-table) |
| 16 | Record `session_meta.git.branch` and `commit_hash` per thread, and `root_turn_id` as a link from subagent requests to a root turn | Queued | [Branch and Agent Grouping](../../urollup-design.md#branch-and-agent-grouping); `root_turn_id` only in brief [Session Linkage](research-2026-09-13-portable-agent-usage.md#session-linkage) |
| 17 | Add synthetic goldens for exec usage over resume, context-full fills, compaction estimates, `info: null`, multi-`limit_id` snapshots, legacy user forks, subagent prefixes, revert files, `.zst` twins, archived renames and migrated rollouts | Applied | `uro-obx5`; plan [Testing Strategy](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#testing-strategy) |
| 18 | Keep observations captured before an in-place migration as evidence and diagnose records missing afterwards | Applied | [Design §2.5](../../urollup-design.md#25-capture-store-and-cache) ([Decision 8](../../urollup-design.md#decision-8-capture-store-and-cache)); `uro-xm48`, `uro-gnqj` |
| 19 | Detect nested agents in hooks by comparing hook `session_id` with any `CODEX_SESSION_ID` | Applied | `uro-20ck`; [design §6.2](../../urollup-design.md#62-current-session-detection) nested-agent rule |
| 20 | Report ephemeral threads, parallel guardian reviews and legacy remote compaction as unobserved, never zero | Applied | [Design §2.1](../../urollup-design.md#21-dialects-and-discovery) and [§3.3](../../urollup-design.md#33-reconciliation); `uro-spce` |

### ccusage Recommendations

| # | Recommendation | Status | Where applied |
| ---: | --- | --- | --- |
| 1 | Credit session-scoped Claude dedupe to `a4b8420`, describe Codex replay handling as value matching with a 1 s burst fallback, and list what 20.0.20 does not read | Applied | Brief [Existing Implementations](research-2026-09-13-portable-agent-usage.md#existing-implementations) and [Reusable Code and Tests](research-2026-09-13-portable-agent-usage.md#reusable-code-and-tests) |
| 2 | Add dialect facts 1–5 and 7 to the Claude rows, 8–14 to the Codex rows and 16 to the Pi linkage row | Partial | Brief [Usage Fields](research-2026-09-13-portable-agent-usage.md#usage-fields) and [Session Linkage](research-2026-09-13-portable-agent-usage.md#session-linkage); [design §3.1](../../urollup-design.md#31-entities), [§4.1](../../urollup-design.md#41-measure-contracts) and [§4.5](../../urollup-design.md#45-price-table); OpenAI’s 272K per-request threshold (fact 14) only in [ccusage Dialect Facts](#ccusage-dialect-facts) |
| 3 | Record that Pi fork copies keep timestamps, and add the Codex MultiAgent V2 replay markers to the verification list | Applied | Brief [Session Linkage](research-2026-09-13-portable-agent-usage.md#session-linkage) and [Next Steps](research-2026-09-13-portable-agent-usage.md#next-steps) |
| 4 | Key Claude requests by provider, `message.id` and `requestId`; treat `isSidechain` replays as copies and gateway reuse as ambiguous; select one whole record by a documented, order-independent rule | Applied | [Design §3.4](../../urollup-design.md#34-dialect-reconciliation-rules) and [Identity Basis and Linking](../../urollup-design.md#identity-basis-and-linking) |
| 5 | Count an advisor iteration as model usage inside one request, and `progress`-embedded messages as observations of the subagent’s request | Applied | [Design §4.1](../../urollup-design.md#41-measure-contracts), [§2.3](../../urollup-design.md#23-capture-layers-and-re-extraction) and [§3.4](../../urollup-design.md#34-dialect-reconciliation-rules) |
| 6 | Treat line prefilters as hints, never reject a record for a nested null, count malformed lines per source, and accept any RFC 3339 precision | Applied | [Design §2.2](../../urollup-design.md#22-snapshot-boundary) |
| 7 | Order Codex replay handling by response identity, start ordinal, verified MultiAgent V2 markers, the foreign-`session_meta` cut, then ccusage’s value match and burst heuristic, each labeled inferred | Partial | [Design §3.4](../../urollup-design.md#34-dialect-reconciliation-rules) has the ordinal, `thread_settings_applied` and squares-derived fallbacks and the epoch rule; the value match and burst fallback only in brief [Reusable Code and Tests](research-2026-09-13-portable-agent-usage.md#reusable-code-and-tests); the markers remain unverified |
| 8 | Model whole-request context bands, store 1-hour cache-write rates, leave missing rates unpriced, take tier only from `thread_settings_applied`, and carry pricing coverage | Applied | [Design §4.1](../../urollup-design.md#41-measure-contracts) and [§4.5](../../urollup-design.md#45-price-table); `uro-wuby` |
| 9 | Honor `$XDG_CONFIG_HOME/claude/projects` and a `CLAUDE_CONFIG_DIR` value ending in `projects/`, include `subagents/workflows/`, follow symlinks within roots, and name ccusage’s `PI_AGENT_DIR` and comma lists in the feature matrix | Partial | [Design §2.1](../../urollup-design.md#21-dialects-and-discovery), [§2.2](../../urollup-design.md#22-snapshot-boundary) and [§3.2](../../urollup-design.md#32-relationships-and-the-discovery-index); the `projects/`-ending value and the feature-matrix note are not recorded |
| 10 | Name the ccusage command path for each feature-matrix row, pin `--offline` and `--mode calculate`, and list expected disagreements | Applied | `uro-o65n`; brief [Existing Implementations](research-2026-09-13-portable-agent-usage.md#existing-implementations) |
| 11 | Calibrate benchmark file sizes against ccusage’s profile, with block-split Claude requests and repeated Codex snapshots | Applied | `uro-sbnk` |
| 12 | For `--current`, parse only the resolved transcript and its `subagents/` directory | Applied | `uro-20ck` |
| 13 | Port ccusage test cases with the MIT notice and add negative fixtures for its bugs | Applied | `uro-obx5`; plan [Testing Strategy](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#testing-strategy) |
| 14 | Add a third-party notices file and a source header convention, and keep unwinding in the release profile | Applied | `uro-phi8`; [Decision 3](../../urollup-design.md#decision-3-code-reuse-and-licensing) and [design §8.2](../../urollup-design.md#82-engineering-conventions) |
| 15 | Port the discovery, tier, cumulative-delta, parallel reader and prefilter logic, and read `.jsonl.zst`, `token_usage_record`, `rate_limits`, `effort`, `quotaLimits`, `progress` records and advisor iterations | Applied | `uro-y2qj` |
| 16 | Confirm in Codex source the MultiAgent V2 markers, history re-stamping, the `archived_sessions/` layout and whether `cache_write_input_tokens` is inside `input_tokens` | Partial | The Codex review confirmed re-stamping and the flat archive layout; the other two remain in brief [Next Steps](research-2026-09-13-portable-agent-usage.md#next-steps) |
| 17 | Build the maintainer price diff script from ccusage’s LiteLLM field list and models.dev authoring-catalog preference, using fast multipliers only as prompts | Partial | [Design §4.5](../../urollup-design.md#45-price-table) plans the diff script; the catalog preference and fast-multiplier use are not recorded |
| 18 | Add a Pi fork fixture that replays the parent’s active branch with original timestamps while an abandoned sibling stays in the parent | Partial | `uro-obx5` and plan [Testing Strategy](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#testing-strategy) cover Pi forks; the abandoned-sibling case is not recorded |

### Pi, agentfdr and Anthropic Plugin Recommendations

These three reviews shared one recommendation list, so the table names the reviews each
item draws on.

| # | Reviews | Recommendation | Status | Where applied |
| ---: | --- | --- | --- | --- |
| 1 | Pi | Record that copies keep entry IDs, that 8-hex entry IDs are unsafe cross-file keys, the tool, compaction and branch-summary usage carriers, the `custom_message`, `label` and `session_info` entries, `settings.json` `sessionDir`, and that files record no Pi version | Applied | Brief [Log Dialects and Session Linkage](research-2026-09-13-portable-agent-usage.md#log-dialects-and-session-linkage); [design §2.1](../../urollup-design.md#21-dialects-and-discovery) and [Key Scope](../../urollup-design.md#key-scope) |
| 2 | Pi | Replace “Pi repeats usage up to five times” with the `pi-events` version bounds, and move `message_final` to the metaproc review | Applied | Brief [Usage Fields](research-2026-09-13-portable-agent-usage.md#usage-fields); [metaproc and qm review](research-2026-09-14-metaproc-code-review.md) |
| 3 | session-report | Correct its request key fallbacks, per-file maximum, first-file-wins rule and order-dependent replay attribution | Applied | Brief [Existing Implementations](research-2026-09-13-portable-agent-usage.md#existing-implementations) |
| 4 | agentfdr | Correct its `iterations` totals, repeated `token_count` double count and omitted subagent files | Applied | Brief [Existing Implementations](research-2026-09-13-portable-agent-usage.md#existing-implementations) |
| 5 | All | Add verification items: receipts’ 13% disagreement, the `<session>/workflows/` layout, entry-level `effort`, and the Pi v4 store | Applied | Brief [Next Steps](research-2026-09-13-portable-agent-usage.md#next-steps) |
| 6 | Pi | Exit 1 for `PI_SESSION_ID` without `PI_SESSION_FILE`, and note that Pi summaries include the calling request | Applied | [Design §6.2](../../urollup-design.md#62-current-session-detection); `uro-20ck` |
| 7 | Pi | List saved `--mode rpc` transcripts as unsupported and the v4 store as a watch item | Applied | [Design §2.1](../../urollup-design.md#21-dialects-and-discovery); brief [Log Dialects and Session Linkage](research-2026-09-13-portable-agent-usage.md#log-dialects-and-session-linkage) |
| 8 | Pi | Scope Pi entry IDs to a lineage, key `thr-` IDs by header `id` and timestamp, and key fallback `req-` IDs by lineage root and a digest | Applied | [Key Scope](../../urollup-design.md#key-scope) and [design §3.6](../../urollup-design.md#36-analytical-identities); `uro-spce` |
| 9 | Pi | Take fork edges from `parentSession` or `branchedFrom` resolved by basename, and treat same-`id` files without a parent as export and import copies | Applied | [Design §3.2](../../urollup-design.md#32-relationships-and-the-discovery-index) |
| 10 | Pi, session-report | Never count nested copies, and treat a Claude subagent record with a parent record’s `uuid` as a parent-owned copy | Applied | [Design §3.3](../../urollup-design.md#33-reconciliation) and [§3.4](../../urollup-design.md#34-dialect-reconciliation-rules) |
| 11 | Pi | Give usage carriers that stand for zero, one or several requests an unknown call count | Applied | [Design §4.1](../../urollup-design.md#41-measure-contracts) |
| 12 | session-report, receipts | Select one Claude block record per request with a diagnostic, never merging fields | Applied | [Design §3.4](../../urollup-design.md#34-dialect-reconciliation-rules); `uro-y2qj` |
| 13 | Pi | Stub `toolResult.details` and `retainedTail` while keeping nested `usage`, and capture `message_end`, `compaction_end`, `auto_retry_*` and the last `message_update` usage | Applied | [Design §2.3](../../urollup-design.md#23-capture-layers-and-re-extraction) and [§2.4](../../urollup-design.md#24-capture-and-export-strip-policies) |
| 14 | Pi, receipts | Require 1-hour cache-write rates and request-wide input tiers where the highest matching threshold wins (data contracts and `uro-wuby`) | Applied | [Design §4.5](../../urollup-design.md#45-price-table); `uro-wuby` |
| 15 | Pi | Implement the Pi dedupe and fork rules, v1 to v3 handling, version-bounded `pi-events` parsing, the `getSessionStats` check and coverage labels | Applied | `uro-qnok` |
| 16 | All | Generate Pi fixtures with Pi’s writer, derive structure-only fixtures from Pi’s v1 files, and add Claude block-record, `uuid` replay, non-`msg_0` and multi-`iterations` fixtures and agentfdr’s repeated `token_count` shape | Applied | `uro-obx5`; brief [Reusable Code and Tests](research-2026-09-13-portable-agent-usage.md#reusable-code-and-tests); plan [Testing Strategy](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#testing-strategy) |
| 17 | agentfdr, receipts | Read `promptId`, `turn_duration`, `server_tool_use`, `speed` and refusals, use entry `effort` first, and keep the last `iterations` element as a context-size measure | Applied | `uro-y2qj` |
| 18 | agentfdr, session-report | Port spawn attribution and inline sidechain nodes, resolve subagent types, and mark `task_summary` and `compact` forks as background forks | Applied | `uro-20ck`; [design §3.2](../../urollup-design.md#32-relationships-and-the-discovery-index) |
| 19 | All | Follow the reporting skill model, and add agentfdr’s detectors and Pi’s cache-miss estimator to `check` as labeled estimates | Partial | Skill model in `uro-jpmf` and brief [Existing Implementations](research-2026-09-13-portable-agent-usage.md#existing-implementations); detectors queued as [Anomaly Detectors](../../urollup-design.md#anomaly-detectors); the cache-miss estimator only in brief [Reusable Code and Tests](research-2026-09-13-portable-agent-usage.md#reusable-code-and-tests) |
| 20 | session-report, receipts, agentfdr | Add session-report and receipts rows to the feature matrix, with agentfdr’s `iterations`, Codex and subagent differences | Open | `uro-o65n` and plan [Milestone 0.5](../specs/active/plan-2026-09-13-urollup-cli-and-web.md#milestone-05-full-phase-1-surface-benchmarks-and-parity) name only ccusage and agentfdr |
| 21 | agentfdr | Evaluate Claude Desktop’s `claude-code-sessions/<accountUuid>/<orgUuid>/` store as an opt-in account source | Open | Not recorded in `uro-um7n` or the design |
| 22 | Pi | Add a Pi file with large `toolResult.details` payloads and a pre-0.84.0 `pi-events` capture to the throughput corpus | Partial | `uro-g8r0` is closed; `uro-sbnk` includes a pre-0.84.0 Pi log, and the large-`details` file is not recorded |
| 23 | Pi | Open a P3 watch item for Pi’s v4 JSONL and SQLite stores, whose `v3-import` aggregate must not count as a request | Partial | [Design §2.1](../../urollup-design.md#21-dialects-and-discovery) lists the store as unsupported and brief [Next Steps](research-2026-09-13-portable-agent-usage.md#next-steps) watches it; no bead exists |

## Next Steps

- [ ] Route the open and partial recommendations above to beads or queued design
  decisions.
- [ ] Verify the still-unverified items listed for each source; the portable brief’s
  [Next Steps](research-2026-09-13-portable-agent-usage.md#next-steps) tracks those that
  affect the design.

## Methodology

All five reviews were read-only source reviews on 2026-09-14, run against partial clones
at the pinned commits in the gitignored `attic/` directory with grep and targeted reads.
No local agent logs were read, and no clone’s working tree was changed.

- **Codex:** grep and targeted reads of `codex-rs/`, plus three parallel read-only
  subagent sweeps (exec JSON; environment, hooks and SQLite; tests and fixtures) whose
  key claims were spot-checked against the source.
  Version bounds come from `protocol.rs`, `spawn.rs`, `exec/src/lib.rs` and
  `tui/src/app_server_session.rs` fetched at release tags through the GitHub contents
  API. `attic/codex` is a partial (promisor) clone, so `git log -S` history searches
  failed on missing objects, and history queries lazily fetched 8 promisor packs into
  `attic/codex/.git/objects/pack/`; the working tree, `HEAD` and refs were unchanged.
- **ccusage:** read `rust/Cargo.toml`; every file of `adapters/claude`,
  `adapters/codex`, `adapters/pi` and `adapters/common`; the core pricing, cost, types,
  dates, output and fast modules; the binary’s commands and blocks; the unified loader;
  the terminal and test-support crates; and the benchmark generator.
  Other adapters were surveyed through their READMEs and dedupe code.
  Diffs of later commits `a4b8420`, `15b3bef` and `809eeb6` and the commit messages of
  `b2809fa`, `527ec3a`, `d39a09d` and `e06c08e` were read with `GIT_NO_LAZY_FETCH=1`, so
  the partial clone fetched nothing, and `git grep` at `95bbc41` checked whether later
  code reads the fields in [dialect fact 15](#ccusage-dialect-facts).
- **ccusage Gemini adapter, 2026-09-15:** every file of `rust/adapters/gemini` and its
  README and guide page at the same 20.0.20 checkout, with the long-context tiering in
  `ccusage-core`’s cost module; the GitHub API listed the adapter’s commits after
  20.0.20 and returned the two patches that touch it, so the partial clone fetched no
  history. Nothing was executed.
- **Synthetic probe:** one probe compiled ccusage’s `has_unsupported_null_field`
  unchanged, with a local `memmem` shim, in a scratchpad and ran it on two synthetic
  records. No other source code was run in any review, and the ccusage review used no
  network access or ccusage binary.
- **Pi, agentfdr and the Anthropic plugins:** the files in the [Scope](#scope) table,
  with conclusions that no source test or doc states labeled inferred.
- **Condensation:** bead `uro-9i57` folded these findings into the portable brief, and
  bead `uro-tjzv` applied factual corrections to the plan and data contracts (now the
  design), recorded implementation findings as notes on 16 beads, and queued design
  decisions in `uro-gxen`. The review notes behind those beads are preserved here,
  reorganized by source, and the recommendation statuses were checked on 2026-09-15.

## References

Sources reviewed, at the pinned revisions:

- [OpenAI Codex `rust-v0.154.0`](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340)
  (Apache-2.0, with a `NOTICE` file)
- [ccusage 20.0.20](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1)
  (MIT) and later commits
  [`a4b8420`](https://github.com/ccusage/ccusage/commit/a4b8420ce6a93dc0fd74e685049e97a9c1d1eb84)
  (session-scoped Claude dedupe),
  [`15b3bef`](https://github.com/ccusage/ccusage/commit/15b3bef85b1e0d440ca98e345b4fb5610a41a195)
  (Codex cache writes),
  [`809eeb6`](https://github.com/ccusage/ccusage/commit/809eeb6d52a2c7d13b9c65e10d4106109247390c)
  (Pi fork replay),
  [`b2809fa`](https://github.com/ccusage/ccusage/commit/b2809fa580962f39483a3a0e3fca937c74de7dcb)
  (session report windows),
  [`527ec3a`](https://github.com/ccusage/ccusage/commit/527ec3a9cefa28391664b5c0c0ce1aa006264769)
  (Codex date-window skipping) and
  [`d39a09d`](https://github.com/ccusage/ccusage/commit/d39a09def5b4fd36369dfc9e710a916126497cba)
  (time-aware DeepSeek rates); its
  [Gemini adapter](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/gemini)
  and
  [Gemini guide](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/docs/guide/gemini/index.md)
  were reviewed on 2026-09-15, with `main` at
  [`d341949`](https://github.com/ccusage/ccusage/commit/d34194988f460fdb9572d138226b9d9380c04a48)
  carrying no Gemini parsing change
- [Pi coding-agent 0.85.1](https://github.com/earendil-works/pi/tree/d981de1229ef899957bbe968bc8dcda02a21f477)
  (MIT)
- [agentfdr 0.8.0](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90)
  (MIT)
- [Anthropic session-report plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/session-report)
  and
  [receipts plugin](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537/plugins/receipts)
  (official plugins, Apache-2.0, no `NOTICE` file; their transcript notes are empirical,
  not a format specification)

Price data that ccusage embeds (community-maintained datasets, MIT):

- [LiteLLM model price file](https://github.com/BerriAI/litellm/blob/1a183efaa1a2108aed7e1bed8d445d93bd1aa60d/model_prices_and_context_window.json)
- [models.dev](https://github.com/anomalyco/models.dev/tree/bff41227803631c84903fcf7f486370e9fbcde86)

Related project documents:

- [Portable agent usage research brief](research-2026-09-13-portable-agent-usage.md)
- [urollup design](../../urollup-design.md)
- [urollup plan](../specs/active/plan-2026-09-13-urollup-cli-and-web.md)
- [squares code review](research-2026-09-14-squares-code-review.md) and
  [metaproc and qm review](research-2026-09-14-metaproc-code-review.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
