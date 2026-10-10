---
title: Codex Paginated Subagent History Boundaries
description: What subagent_history_start_ordinal means in Codex rust-v0.155.0 through rust-v0.162.1, read from source, including the legacy-to-paginated migration that puts the boundary past a rollout's last line, and the ownership rule urollup takes from it.
date: 2026-10-10
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete for rust-v0.155.0 through rust-v0.162.1; the residual risks below need a real-history check
---
# Research: Codex Paginated Subagent History Boundaries

## Overview

PR #16 (`uro-kpbp`) treated every line before a Codex child rollout’s
`subagent_history_start_ordinal` as history copied from its parent.
A local scan of real history then found Guardian review and subagent rollouts whose
declared boundary lies past their last line, with the child’s own `token_usage_record`
lines before it, so their usage was excluded as copies while coverage stayed complete
(`uro-jqc3`).

This note reads the public Codex source to settle what the ordinal indexes, what a
paginated child file holds before it, and how urollup should assign ownership.
It was read from shallow, sparse fetches of `rust-v0.162.1` (commit `092d3ac`, the
newest release) and `rust-v0.155.0` (commit `f0a1b8f`); the code cited below is the same
in both unless noted.
Codex is Apache-2.0; this note paraphrases rather than quotes it.
The earlier survey is in
[research-2026-09-14-agent-tool-source-reviews.md](research-2026-09-14-agent-tool-source-reviews.md#subagents-forks-and-resume).

## Findings

### Ordinals Number the Child’s Own File

- A paginated rollout numbers its lines from 0, or from its `history_base` end when it
  references an earlier file (a revert), so an ordinal is a position in the thread’s own
  history, never the parent’s
  ([`rollout/src/ordinal.rs:23-33`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/rollout/src/ordinal.rs#L23-L33)).
  The `session_meta` is the first line.
- `subagent_history_start_ordinal` indexes that same space in both places that write it.
  Its meaning differs between them, as the next two sections show.

### Native Paginated Children

- A paginated thread whose `thread_source` is `subagent` or `guardian_review` and that
  is forked with copied history persists the inherited items when it is created
  ([`core/src/session/session.rs:862-868`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/session.rs#L862-L868),
  [`1074-1085`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/session.rs#L1074-L1085)).
  The boundary is the number of persisted inherited items plus one, so those items sit
  at ordinals 1 through boundary − 1
  ([`thread-store/src/live_thread.rs:155-177`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/live_thread.rs#L155-L177)).
- The child then appends only its own `thread_settings_applied`, which lands at the
  boundary
  ([`core/src/session/mod.rs:1704-1716`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/mod.rs#L1704-L1716)).
  Codex refuses to resume a child whose last line is before boundary − 1
  ([`rollout/src/ordinal.rs:86-96`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/rollout/src/ordinal.rs#L86-L96)).
- **Subagent prefixes carry no usage.** A fork never copies `token_usage_record`, a
  paginated destination also drops `token_count` and `thread_settings_applied`, and a
  copied `compacted` line loses its `latest_token_usage_record`
  ([`core/src/agent/control/spawn.rs:88-132`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/agent/control/spawn.rs#L88-L132),
  [`1209-1233`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/agent/control/spawn.rs#L1209-L1233)).
  The parent’s `session_meta` is kept, as a foreign header.
- **Guardian prefixes** hold either the parent’s compaction item
  ([`core/src/guardian/review_session_setup.rs:122-134`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/guardian/review_session_setup.rs#L122-L134))
  or an earlier reviewer’s checkpoint: a `compacted` line whose
  `latest_token_usage_record` is that reviewer’s latest record, optional world-state and
  turn-context lines, and a `token_count` with that reviewer’s running total, whose last
  usage is the same response
  ([`core/src/session/guardian_checkpoint.rs:11-41`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/guardian_checkpoint.rs#L11-L41)).
- So a native prefix never names the child: no settings event, record or compacted
  record in it carries the child’s thread ID.

### Migrated Legacy Children

Codex’s legacy-to-paginated migration rewrites a legacy rollout in place
(`thread-store/src/local/rollout_migration.rs`, present since `rust-v0.155.0`).

- It runs from `codex migrate-rollouts --apply`
  ([`cli/src/main.rs:217-218`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/cli/src/main.rs#L217-L218))
  or at startup under the `background_paginated_rollout_migration` feature, which is
  under development and off by default but which an app-server client can switch on at
  runtime
  ([`features/src/lib.rs:1254-1259`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/features/src/lib.rs#L1254-L1259),
  [`app-server/src/request_processors/config_processor.rs:305-333`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/app-server/src/request_processors/config_processor.rs#L305-L333)).
- A rollout whose `source` is any subagent source, Guardian included, or whose
  `thread_source` is `subagent`, migrates as a subagent
  ([`rollout_migration.rs:466-473`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L466-L473)).
- The head `session_meta` is rewritten at ordinal 0 as paginated with no boundary, and
  every later `session_meta` line is dropped, so a legacy copied prefix loses the
  foreign header that marked it
  ([`rollout_migration/canonicalizer.rs:82-100`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L82-L100),
  [`117`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L117)).
- A subagent keeps either its whole rollout or only the suffix from its newest
  `compacted` line that has replacement history and a window number; every line of such
  a suffix gets the `session_meta` timestamp
  ([`rollout_migration/subagent.rs:26-53`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/subagent.rs#L26-L53),
  [`rollout/src/model_context.rs:32-49`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/rollout/src/model_context.rs#L32-L49),
  [`rollout_migration.rs:874-914`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L874-L914)).
- **The boundary is then set one past the last migrated line**
  ([`rollout_migration.rs:714-731`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L714-L731),
  [`rollout_migration/publish.rs:156-178`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/publish.rs#L156-L178)),
  and Codex’s own projection treats every line before it as inherited
  ([`rollout_migration.rs:1154-1172`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L1154-L1172)).
  Here the boundary marks where a resumed child appends, not where the child’s own lines
  start: the child’s own settings events, records and counters all sit before it, after
  any copied prefix.

This matches the scanned shape: a paginated Guardian rollout that numbers its lines from
0, starts with a compaction and a settings event naming itself, holds its own records,
and declares a boundary past its end.

### Usage Records Name Their Thread

- Codex writes the recording thread into each `token_usage_record` when the response
  completes, and a copy keeps that ID
  ([`protocol/src/protocol.rs:2293-2302`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/protocol/src/protocol.rs#L2293-L2302),
  [`core/src/session/mod.rs:4720-4752`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/mod.rs#L4720-L4752)).
- The record is written before the `token_count` that reports the same response
  ([`core/src/session/turn.rs:2969-2983`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/turn.rs#L2969-L2983)).

## Ownership Rule

urollup applies this as of `uro-jqc3` (design §3.4):

1. A `token_usage_record` that names a thread belongs to it, wherever it sits: an
   original when it names the rollout’s own thread, a copy otherwise.
   The boundary, its validity and the record’s ordinal do not change that.
2. A line that names no thread (a `token_count`, or a record without `thread_id`)
   follows the latest earlier line that names one: another thread’s `session_meta`, a
   `thread_settings_applied`, a usage record, or a `compacted` line’s record.
   So a counter follows the record it reports, and a Guardian checkpoint’s counter
   follows the earlier reviewer.
3. Before a declared boundary, an unnamed line with no such earlier line is the parent’s
   inherited prefix, as PR #16 decided.
   The rollout’s own header does not count, because it precedes the native prefix.
4. When no line reaches the boundary and no line names the rollout’s thread, counter
   usage before the boundary cannot be placed: it may be a migrated child’s own usage or
   a copied prefix. It is excluded with a `codex-history-boundary-unverified` diagnostic
   and a coverage gap rather than counted or silently treated as a copy.

PR #16’s other protections are unchanged: a foreign `session_meta`, records naming the
parent, the first-step check against the inherited total, and the exclusion of unnamed
usage under an invalid or unpositioned boundary.

## Residual Risks

- **Migrated counter-only children.** A child whose legacy content predates
  `token_usage_record` (0.153) and whose settings events carry no thread ID (before
  0.152) has nothing that names it, so rule 4 reports its counter usage as a coverage
  gap instead of counting it.
- **Migrated user forks.** A legacy user fork migrates as an ordinary rollout: it keeps
  no boundary and loses the foreign header of its copied prefix.
  Copied records still name the parent, but copied counters in a counter-only fork can
  read as the fork’s own unless the parent’s settings events name the parent.
  This is unchanged by `uro-jqc3` and needs a real-history check.
- **Usage dropped by migration.** A bounded subagent migration drops the child’s records
  before its newest compaction; only that `compacted` line’s record survives, as a copy.
  urollup cannot recover the rest, and the surviving records carry the session start as
  their timestamp.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
