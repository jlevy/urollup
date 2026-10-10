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
  ([`cli/src/main.rs:218-219`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/cli/src/main.rs#L218-L219))
  or at startup under the `background_paginated_rollout_migration` feature, which is
  under development and off by default but which an app-server client can switch on at
  runtime
  ([`features/src/lib.rs:1254-1259`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/features/src/lib.rs#L1254-L1259),
  [`app-server/src/request_processors/config_processor.rs:305-333`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/app-server/src/request_processors/config_processor.rs#L305-L333)).
- A rollout whose `source` is any subagent source, Guardian included, or whose
  `thread_source` is `subagent`, migrates as a subagent, unless it is a memory
  consolidation
  ([`rollout_migration.rs:458-473`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L458-L473)).
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
- `turn_context` lines pass through unchanged
  ([`rollout_migration/canonicalizer.rs:291-298`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L291-L298)),
  and a `turn_context` carries `turn_id` from `rust-v0.100.0` on.
  A legacy subagent copied its parent’s rollout, so its copied turns are turns the
  parent recorded, and its own turns are new.

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
   inherited prefix, as PR #16 decided, in a rollout that holds another thread’s
   `session_meta` (a native subagent prefix keeps its parent’s) or has usage records.
   The rollout’s own header does not count, because it precedes the native prefix.
4. In a counter-only rollout without another thread’s `session_meta`, turns decide those
   lines, region by region: inside a turn the parent root recorded they are its copy,
   and the first turn it never recorded starts the child’s own lines, whose first step
   is still checked against the copied total.
   A counter that no turn places is a copy when the parent root, itself counter-only,
   reported its cumulative total.
5. Unnamed usage (counters and records without a `thread_id`) that rules 2 to 4 leave
   undecided is excluded with a `codex-history-boundary-unverified` diagnostic and a
   coverage gap, rather than counted or silently treated as a copy, when its region
   shows migrated content: a turn without an ID, a parent that is not a discovered root,
   or a boundary no line reaches.
   Otherwise it is inherited, as in a native prefix whose own settings event reaches the
   boundary. The diagnostic’s occurrences count these undecided steps, unverified first
   steps and running totals beyond the child’s records, never steps decided as copies.
6. A `compacted` line’s record that names the rollout’s own thread, when no usage record
   in the rollout reports the same response, is that response’s only record left by a
   bounded migration, so it is the original; any other original merges with it by
   response key. In a rollout with usage records, a first running total after it beyond
   what the child’s own records report shows responses the migration dropped, which is a
   gap.

PR #16’s other protections are unchanged: a foreign `session_meta`, records naming the
parent, the first-step check against the inherited total, and the exclusion of unnamed
usage under an invalid or unpositioned boundary.

## Residual Risks

- **Undecidable migrated children.** A migrated counter-only child whose parent is not a
  discovered root, or whose turns have no IDs (before 0.100), or whose counters precede
  every turn and match no parent total, reports its undecided usage as a coverage gap
  instead of counting it.
  Only counter-only roots supply totals, so a child of a root with usage records gets no
  total match.
- **Children with usage records and counter-only turns.** A migrated or resumed child
  with usage records is accounted from its records alone, so counter-only turns before
  them are not counted (`uro-r8si`).
- **Migrated user forks.** A legacy user fork migrates as an ordinary rollout: it keeps
  no boundary and loses the foreign header of its copied prefix.
  Copied records still name the parent, but in a counter-only fork the copied counters
  count as the fork’s own (`uro-p9ua`). The turn and total evidence of rule 4 would
  decide them too, from the first line, but this is unchanged by `uro-jqc3`.
- **Usage dropped by migration.** A bounded subagent migration drops the child’s lines
  before its newest compaction.
  Only that `compacted` line’s record survives, and the surviving lines carry the
  session start as their timestamp, which moves their usage to that day.
  The dropped responses are a gap only where the running total shows them: a total that
  compaction lowered can hide them, and a legacy child seeded with its parent’s total
  reports a gap even when the migration dropped no other response.
- **Native settings order.** Rule 2 lets a line that names the child before the boundary
  override it. Codex 0.152 through 0.162.1 writes a native child’s settings event at the
  boundary, after its inherited prefix; a writer that put it first would make the prefix
  count as the child’s.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
