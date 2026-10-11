---
title: Codex Paginated Subagent History Boundaries
description: What subagent_history_start_ordinal means in Codex rust-v0.155.0 through rust-v0.162.1, read from source, including the legacy-to-paginated migration that puts the boundary past a rollout's last line and leaves a legacy user fork's copied prefix unmarked, and the ownership rules urollup takes from it.
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
(`uro-jqc3`). Review of that fix found the opposite error in migrated legacy user forks,
which declare no boundary: their copied counters counted again (`uro-p9ua`).

This note reads the public Codex source to settle what the ordinal indexes, what a
paginated child file holds before it, and how urollup should assign ownership.
It was read from shallow, sparse fetches of `rust-v0.162.1` (commit `092d3ac`, the
newest release) and `rust-v0.155.0` (commit `f0a1b8f`); the code cited below is the same
in both unless noted.
The user-fork findings were read at `rust-v0.162.1` only, with the referenced fork also
checked at `rust-v0.150.0`. Codex is Apache-2.0; this note paraphrases rather than
quotes it. The earlier survey is in
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

### Migrated Legacy User Forks

- A legacy user fork copies its parent’s rollout, `session_meta` and `token_count`
  events included
  ([survey](research-2026-09-14-agent-tool-source-reviews.md#subagents-forks-and-resume)).
  The migration treats it as an ordinary rollout, since only a subagent source or a
  `subagent` `thread_source` makes a subagent
  ([`rollout_migration.rs:458-473`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L458-L473)),
  and only a subagent gets a boundary
  ([`729-731`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration.rs#L729-L731)).
- So a migrated fork keeps its `forked_from_id`, its head becomes paginated with no
  `history_base` and no boundary
  ([`canonicalizer.rs:82-100`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L82-L100)),
  and the copied prefix’s `session_meta` is dropped
  ([`117`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/rollout_migration/canonicalizer.rs#L117)).
  Nothing marks the copied lines any more, and their `turn_context` lines keep the
  parent’s turn IDs.
- A native paginated user fork copies nothing: it references its parent’s prefix
  ([`core/src/thread_manager.rs:1502-1517`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/thread_manager.rs#L1502-L1517)),
  writes its own settings event first and then its own lines
  ([`core/src/session/mod.rs:1692-1703`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/mod.rs#L1692-L1703)),
  and declares a `history_base`, unless it forks at the start of its parent’s first
  rollout. There the history base is none
  ([`thread-store/src/local/paginated_fork.rs:170-178`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/paginated_fork.rs#L170-L178)),
  and the inherited context is only the parent’s header
  ([`model_context.rs:149-155`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/local/model_context.rs#L149-L155)).
- **A native fork’s first counter is seeded.** Codex seeds a fork’s running total from
  the last `token_count` in its inherited context
  ([`core/src/session/mod.rs:1683-1688`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/mod.rs#L1683-L1688)),
  so a native fork with a history base starts at its parent’s latest total when its
  inherited context holds a counter.
  Referenced forks are present at `rust-v0.150.0` (`3b3b4f8`), so one written before
  0.153 holds counters only.
  A migrated fork’s first counter instead copies one of its parent’s.
- A native paginated subagent gets a boundary exactly when it inherits context
  ([`thread-store/src/live_thread.rs:151-177`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/thread-store/src/live_thread.rs#L151-L177),
  [`core/src/session/session.rs:862-868`](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/session/session.rs#L862-L868)),
  so a paginated subagent without one inherited nothing.

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
   reported its cumulative total, with a missing count read as 0. A parent root here is
   a thread none of whose rollouts carries lineage (a top-level parent or fork link, a
   boundary or another thread’s header) or a `source.subagent.thread_spawn` parent.
   Only such a rollout can be a complete record of what the child copied.
   After a turn-inferred start, a later turn the parent recorded, a counter total it
   reported other than the copied total, or a line naming another thread contradicts the
   inference, and the region from the start to the boundary becomes undecided.
   The start stays open until a line at or past the boundary, or a line naming the
   child’s own thread, which ends the window without voiding it.
   A settings event without a thread ID (written before 0.152) names no owner, so as of
   `uro-p9ua` it leaves an open start’s lines the child’s.
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
   gap. In a counter-only rollout, a counter that reports the same response is its twin
   and adds no usage.
7. As of `uro-p9ua`, a rollout that declares no boundary but may be a migrated user fork
   is judged by rules 4 and 5 as if its boundary lay past its last line: a counter-only
   paginated rollout with a parent link and no other thread’s `session_meta`, without a
   mark of a rollout that copied nothing.
   The marks are a `history_base`; a `thread_source` of `subagent` or `guardian_review`,
   or a subagent `source` other than memory consolidation, as the migration tests it;
   and a seeded first counter, whose total neither its own last usage nor a total its
   parent root reported accounts for.
   The parent’s totals keep a copied counter in the rule when a migrated root’s rollback
   planning dropped its first turn and left that turn’s usage in the next counter; only
   a counter-only root supplies them.
   So its lines are the parent root’s copy from the first line until the first turn the
   parent never recorded, whose first step is checked against the copied total.
   No line reaches the boundary, so an inferred start stays voidable to the end of the
   file unless a line names the fork, and what nothing decides is a gap.

PR #16’s other protections are unchanged: a foreign `session_meta`, records naming the
parent, the first-step check against the inherited total, and the exclusion of unnamed
usage under an invalid or unpositioned boundary.

## Residual Risks

- **Old-format subagent parents.** Before Codex wrote a top-level `parent_thread_id`
  (absent at `rust-v0.130.0`, present at `rust-v0.150.0`), a spawned subagent named its
  parent only in `source.subagent.thread_spawn.parent_thread_id`, which Codex itself
  reads and the migration leaves in place.
  urollup keeps such a thread from deciding a child’s turns, without changing how its
  own counters are read, but does not yet read the link as a parent, so a migrated child
  of one is undecided (`uro-3b12`).

- **Rolled-back spawn turns.** Codex’s migration replays a legacy root through a
  rollback plan that drops a rolled-back turn’s `turn_context` and counters, so the
  root’s next counter step carries that turn’s usage.
  A subagent spawned in that turn still holds its copy, so its inferred own start counts
  that usage again (`uro-eh0d`). `main` does the same for an unmigrated child through
  the legacy turn switch.
  Rule 7 extends this to a migrated user fork taken before the parent rolled back a turn
  it copied: in a synthetic case it reports 280 against a real 220, where `main` reports
  380 (`paginated_forks.rs` pins it).
  A local real-history check found no migrated user fork with own totals inside a parent
  rollback window.

- **Turn evidence beyond roots.** Turns are checked only against complete roots.
  A turn index over every rollout would decide nested chains and siblings too
  (`uro-r8si`).

- **Undecidable migrated children.** A migrated counter-only child whose parent is not a
  discovered root, or whose turns have no IDs (before 0.100), or whose counters precede
  every turn and match no parent total, reports its undecided usage as a coverage gap
  instead of counting it.
  Only counter-only roots supply totals, so a child of a root with usage records gets no
  total match.

- **Children with usage records and counter-only turns.** A migrated or resumed child
  with usage records is accounted from its records alone, so counter-only turns before
  them are not counted (`uro-r8si`).

- **Undecidable migrated forks.** Rule 7 decides a migrated counter-only user fork only
  against a parent root without lineage.
  Without it, a fork of a fork or of a subagent, or one whose turns have no IDs, reports
  every unnamed step as a coverage gap, and so does every step after evidence voids an
  inferred start. A native fork from a thread’s start, written before 0.152 (no
  `history_base` and no settings event naming it), is indistinguishable from a migrated
  fork, so it is a gap when its parent is absent although it copied nothing.
  When a migrated parent’s rollback planning dropped its first turn and the parent is
  absent, holds usage records or has lineage, no counter-only root supplies the totals
  that show the fork’s first counter is a copy, so it looks seeded and the fork counts
  that copy as its own usage, as before `uro-p9ua`. With the parent present, that copy
  counts twice (`uro-eh0d`; a synthetic case reports 350 against a real 190). A fork’s
  own step that repeats a total its parent reported, in every count, as a retried prompt
  can, voids its inferred start and is decided as the parent’s copy, so it is dropped
  with complete coverage; PR #32’s rule does the same for a migrated subagent
  (`paginated_forks.rs` pins both).

- **Native counter-only forks count their seed.** A referenced fork written before 0.153
  has no usage records, and its first counter continues the parent’s total that Codex
  seeded it with. urollup starts a fork that has neither a boundary nor copied counters
  from zero, so that step counts the parent’s total again, with complete coverage.
  Rule 7 deliberately leaves such a fork, marked by its `history_base` and seeded first
  counter, unchanged (`uro-p12c`, which `paginated_forks.rs` pins); checking the seed
  against the parent’s totals would fix it.
  A migrated fork resumed by 0.153 or later falls out of rule 7 once it has usage
  records, and its counter-only turns are not counted (`uro-r8si`).

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
