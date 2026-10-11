# Codex Rollout Contract

## Support and Evidence

Runtime dialect: `codex-rollout`. The
[adapter](../../crates/urollup-core/src/adapters/codex_rollout.rs) and
[typed decoder](../../crates/urollup-core/src/adapters/codex_rollout/line.rs) read
persistent rollouts.
`codex exec --json` is a different stream and is not an implemented input adapter.
Its cumulative usage must not be interpreted as per-turn usage.

The
[source review](../project/research/research-2026-09-14-agent-tool-source-reviews.md)
pins Codex producer evidence to `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`
(`rust-v0.154.0`), including earlier format transitions, and the
[paginated-subagent research](../project/research/research-2026-10-10-codex-paginated-subagent-boundary.md)
covers the later legacy-to-paginated rollout migration.
Direct `token_usage_record` evidence begins with `rust-v0.153.0`; older formats use
cumulative events. These are evidence points, not a tested compatibility claim for every
intervening or later release.

## Discovery and Framing

Codex homes expand to `sessions/` and `archived_sessions/`. Explicit rollout roots and
files are also supported through source selection.
JSONL and compressed twins follow the
[shared reader contract](README.md#shared-reader-contract).

File names supply a discovery identity; `session_meta` supplies native thread metadata.
Archived files are not a separate account.
A new rollout file does not by itself imply new usage.
Preserve timestamps as evidence, but never use a copied record’s new write time to
assign its usage to a child.

## Record Mapping

Paths below `payload` are relative to that object unless stated otherwise.

| Record | Fields relied on | Interpretation |
| --- | --- | --- |
| `session_meta` | `id`, `cli_version`, `source` (including `source.subagent.thread_spawn.parent_thread_id`), `thread_source`, `cwd`, `parent_thread_id`, `forked_from_id`, `subagent_history_start_ordinal`, `model_provider` | Thread, project, provider and ancestry evidence; the ordinal is where the child’s own history starts, and it places only lines that name no thread |
| `turn_context` | `turn_id`, `model`, `effort` | Requested model and effort, not proof of the served model; turn identity for copied-prefix inference |
| `token_usage_record` | `thread_id`, `response_id`, `turn_id`, `root_turn_id`, `usage` | One response, keyed by `response_id` and owned by its `thread_id` wherever it sits, before a declared boundary included; `turn_id`, else `root_turn_id`, selects the turn context |
| `compacted` | `latest_token_usage_record` | Copy evidence, except when it names the rollout’s own thread and no usage record in the file reports its response: a bounded migration left it as the only record of that response, so it counts as the original, and any other original merges with it by response key |
| `event_msg` / `token_count` | `info.total_token_usage`, `info.last_token_usage`, `rate_limits` | Cumulative counter accounting where the counter rules apply, twins of adjacent usage records, and separate provider-limit observations |
| `event_msg` / `thread_settings_applied` | `thread_id`; `thread_settings.service_tier`, `thread_settings.model_provider_id` | The thread whose unnamed lines follow; that thread’s complete settings snapshot, whose selected tier and provider pricing keeps |
| Top-level record | `ordinal`, `timestamp` | Position relative to a declared boundary and native time evidence |

A usage record takes its model, effort and pricing context from the `turn_context` of
its own `turn_id`, and from its `root_turn_id` only when it has no `turn_id`: a
multi-agent subagent’s records name the parent’s turn as their root.
A counter takes the context of the latest `turn_context` with a `turn_id` before it.
Delayed direct records therefore keep their turn’s context.

The pricing context of a turn is the provider and selected tier its thread had when the
`turn_context` was written:

- **Snapshots:** each settings update writes the thread’s complete settings snapshot
  ([`thread_settings.rs:89-96`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/thread_settings.rs#L89-L96)),
  whose `service_tier` is omitted when no tier is selected
  ([`protocol.rs:2194-2198`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2194-L2198)).
  Clearing the tier within a session writes `"default"`
  ([`step_settings.rs:277-286`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/step_settings.rs#L277-L286)),
  and a resume without a tier writes a snapshot without the key
  ([`persisted_resume_settings.rs:14-46`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/app-server/src/request_processors/persisted_resume_settings.rs#L14-L46)).
  So a snapshot without `service_tier`, or with a null one, clears the tier, and an
  event without a snapshot object changes nothing.
  ccusage instead keeps the previous tier when the key is absent
  ([source review](../project/research/research-2026-09-14-agent-tool-source-reviews.md#ccusage-codex-parsing)).
- **Provider:** `session_meta.model_provider` is written once, when the rollout is
  created
  ([`recorder.rs:874-890`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L874-L890)),
  and a later snapshot’s `model_provider_id` replaces it, as after a resume with a
  provider override.
- **Placement:** a snapshot that names a thread sets that thread’s tier and provider
  wherever it sits. One that names no thread sets the rollout’s own: anywhere in a
  rollout without a declared boundary, unless it is a migrated user fork judged as if
  its boundary lay past its last line, and otherwise only at or after the boundary,
  never before it, on a line without an ordinal, or under an invalid boundary.
  So it never sets the tier of such a fork, or of a migrated child whose boundary lies
  past its end.
- **Basis:** the tier is the thread’s selected tier, kept with the basis
  `TierBasis::ThreadSetting`, not the tier a request was sent with.
  Codex filters the selected tier by its FastMode feature and model support
  ([`step_settings.rs:47-49`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/step_settings.rs#L47-L49),
  [`mod.rs:984-996`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L984-L996)),
  applies a per-turn override only to the turn’s copy after persisting the snapshot
  ([`turn_context.rs:871-874`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/turn_context.rs#L871-L874)),
  sends a thread-spawn subagent’s steps with its root’s current tier
  ([`mod.rs:3533-3549`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/mod.rs#L3533-L3549)),
  and leaves future thread settings unchanged on a turn-scoped update
  ([`step_activation.rs:338-343`](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/session/step_activation.rs#L338-L343)).
  How a rate match treats that basis is decided under `uro-wuby`.

Tier comes from recorded events, never the user’s configuration, and an unknown tier
string is kept as recorded, never mapped to standard.
Distinguishing a malformed pricing field from an absent one, and context for usage with
no turn, remain open under `uro-381f`.

## Accounting Rules

A usage record, settings event or `compacted` record that names a thread belongs to, or
assigns the unnamed lines after it to, that thread wherever it sits.
Before an explicit boundary `B`, a line that names no thread (a `token_count`, or a
record without `thread_id`) and that no earlier line assigns is inherited copy evidence,
even when the file contains only the child’s own session header.
A natively created child’s prefix never names the child, while Codex’s
legacy-to-paginated migration puts `B` one past the last line it rewrote, so a migrated
child’s own settings events and records precede `B` and assign its lines to it.
In a counter-only child that holds no other thread’s `session_meta`, the turns its
complete parent root recorded decide the unassigned lines before `B`: a line in a turn
the parent recorded is the parent’s copy, and the first turn the parent never recorded
starts the child’s own lines.
A later parent turn, a parent counter total or a line naming another thread before `B`
voids that inferred start; a line naming the child ends the window without voiding it.
Guardian reviews follow the same rules: a migrated review owns its records before a
boundary past its last line, and a native review’s inherited prefix stays copied.
A known parent can own a copy; otherwise ownership stays unknown.
Direct usage whose native owner differs from the file thread is a copy.
[Design §3.4](../urollup-design.md#34-dialect-reconciliation-rules) states the
precedence, including what makes a parent a complete root.

Legacy counters require the inherited total at the fork, not the parent’s latest total
when the report runs.
Seed the child’s counter with that baseline and count subsequent deltas.
In a child with an explicit boundary, including a boundary of 0, or after copied
counters, the first own counter step that reports usage must match its
`last_token_usage`: as its delta from the inherited total (a seeded child) or as its
whole total (an unseeded child; when the child had an inherited total, this opens a new
epoch with `codex-counter-epoch-reset`, and otherwise it is an advance from zero).
A first step that matches neither is excluded, including the first step of an
explicit-boundary child with no copied counter whose total differs from its
`last_token_usage`; later steps count from its total.
A step that only repeats the inherited total reports no usage, so the step after it is
the one checked, and so does a first step whose total falls below the inherited total
with zero `last_token_usage`, the next step then being checked from the lowered total.

Repeated totals add nothing.
A decrease in any cumulative component opens a new epoch with a diagnostic, and the
decreasing record counts its own `last_token_usage`, not the new total, because Codex
lowers its running total at compaction instead of restarting it from zero.
A decrease whose `last_token_usage` has zero input and output adds no request.
[Design §3.4](../urollup-design.md#34-dialect-reconciliation-rules) states how this rule
meets the first-step check.

A root rollout, one with no parent, fork origin, boundary, other thread’s header, or
settings event or usage record naming another thread, can hold counter-only turns beside
its usage records when releases before and after 0.153 wrote it.
A nested `thread_spawn` parent link alone, or a `compacted` record naming another
thread, does not stop a rollout being a root.
There every `token_count` whose total changes and whose `last_token_usage` reports input
or output goes through the counter rules, unless it is the twin of an adjacent usage
record: the record just before it, or failing that the one just after it, with every
native usage field equal, not taken by an earlier counter, and with no `turn_context`
between them. A twin adds no request but still moves the running total.
Every other rollout with usage records is accounted from its records alone; counter-only
turns in such a fork or subagent are not yet counted (`uro-r8si`). In a rollout with
usage records, a `token_count` inside a copied prefix is a copy keyed to the copied
thread’s last response when an earlier line of the file gave that response’s ID (a usage
record, or the record a Guardian checkpoint’s `compacted` line keeps), and otherwise an
unkeyed copy.
Legacy inference without an ordinal boundary uses foreign headers, settings
and known turn identities; see
[design §3.4](../urollup-design.md#34-dialect-reconciliation-rules).

| Native usage field | Normalized meaning |
| --- | --- |
| `input_tokens` | Inclusive input, containing ordinary input, cache reads and cache writes |
| `cached_input_tokens` | Cache-read input |
| `cache_write_input_tokens` | Cache-write input with unspecified lifetime |
| `output_tokens` | Output, including reasoning |
| `reasoning_output_tokens` | Subset of output; never added again |
| `total_tokens` | Native total used by counter identity/estimate handling, not an extra additive category |

Ordinary input is `input_tokens - cached_input_tokens - cache_write_input_tokens`, with
a missing category read as 0. That cache writes sit inside `input_tokens` is an
inference: Codex reads both cache counts from the Responses API’s `input_tokens_details`
and states only that the cached count is a subset of input; the
[incident record](accounting-incidents.md#codex-cache-write-double-counting) gives the
evidence. The subtraction is checked: a usage object whose `input_tokens` is below the
two cache categories is inconsistent, never a silent zero (see below).
Do not apply Claude’s five-minute cache-write assumption to Codex automatically.

## Failure and Coverage Behavior

Unnamed usage that a declared boundary cannot place is excluded, never counted as the
child’s: an invalid `subagent_history_start_ordinal` places none of the rollout’s
unnamed usage, and a cumulative total or a record without `thread_id` that has no
`ordinal` is not placed.
A record that names its thread needs no placing.
A `token_count` that carries no usage needs no ordinal: one that reports only rate
limits, or one whose total repeats the running total.
The same exclusion covers an unverifiable first counter step, unnamed usage in a
migrated region that nothing decides (in a turn the parent’s turns cannot place, because
it has no ID or the parent is not a complete discovered root, or before a boundary no
line reaches), every step of a voided inferred start, and a running total beyond the
child’s own records after a `compacted` record that is the only record of its response.
Each rollout with excluded usage gets one `codex-history-boundary-unverified`
diagnostic, whose occurrences count the excluded steps, and a coverage gap for its
thread, so that thread and the whole history report partial coverage; the run continues
and every other session still reports.

A usage object whose `input_tokens` is below its cache reads plus writes is excluded the
same way where the rollout would count it: an original record or `compacted` record, or
the total or last usage of an own step the counter rules read.
The excluded record or step adds no usage, and the rollout gets one
`codex-inclusive-input-underflow` diagnostic and a coverage gap for its thread.
The diagnostic’s detail names the rollout’s path, the physical line of the first
excluded record or step, counting blank and malformed lines, and that line’s counts.
Its occurrences count the records and steps whose usage is excluded; a rate-limit
refresh that repeats an excluded step, or the counter of an excluded `compacted`
original, adds none.
A copy with such usage, including a copied counter step before a boundary, keeps only
its evidence and gives the child no gap; a step the boundary cannot place is excluded
once, as unverified.

An inconsistent cumulative step ends its counter epoch: the next own step is checked as
a first step against the last total the rollout could read, and a step that cannot be
checked is excluded with it, so usage before the inconsistent step never counts again.
The same holds after an unreadable copied total, or an unreadable total of a root
rollout’s twin, whose usage is its record’s. In a rollout with no parent, boundary or
other thread’s header, a step after an inconsistent one that lowers the total counts its
own `last_token_usage`, as the counter rules’ reset does.
A total that cannot be read never shows that a migration dropped nothing, so the
dropped-response check excludes it as unverified.

A copy without its original is excluded from counted totals; the Codex adapter emits no
diagnostic for it, and it does not change completeness.
Whether it should is the open decision `uro-xpd0`. Unknown ownership is not silently
assigned to the current file.
A rollout that cannot be read completely makes coverage partial through
`source-incomplete` ([shared reader contract](README.md#shared-reader-contract)).

Zero input and output with a positive native total is a compaction estimate
(`codex-estimate-compaction`) or a context-window fill
(`codex-estimate-context-window-fill`): a diagnostic, not a request.
A rollout whose copied history was inferred from legacy records without an ordinal
boundary gets a `codex-copied-history-inferred` diagnostic.
Rate-limit percentages are not token usage, invoices or list-price estimates.

## Regression Coverage and Open Work

- [Paginated forks](../../crates/urollup-core/tests/paginated_forks.rs): direct/counter
  copies, parent presence, unknown parent, resets, malformed boundaries, missing
  ordinals/baselines, first-step checks around lowered totals, migrated children and
  turn inference, and worker/source-order parity.
- [Mixed usage](../../crates/urollup-core/tests/codex_mixed_usage.rs): counter-only
  turns beside usage records, twins and root-rollout classification.
- [Fixture corpus](../../crates/urollup-core/tests/fixtures/README.md): legacy repeated
  counters, archives, zstd compression, identity cases, `token-usage-records`,
  `mixed-counter-direct`, `paginated-counter-prefix`, `unverified-fork-boundary`,
  `guardian-migrated-boundary`, `guardian-native-boundary`, `compaction-lowered-total`,
  `ambiguous-owner` and `cache-write-input`, with expected accounting and per-request
  model and effort.
- [Process tests](../../crates/urollup/tests/cli_process.rs): gzip and zstd logs report
  exactly like plain logs, and damaged compressed rollouts and unverifiable fork
  boundaries do not stop a report.
- [Inconsistent usage](../../crates/urollup-core/tests/codex_inconsistent_usage.rs):
  records, counter steps, refreshes, twins, compacted originals, copied and unplaced
  steps below their cache categories, across worker counts and discovery orders; the
  [paginated fork tests](../../crates/urollup-core/tests/paginated_forks.rs) cover an
  unreadable total at the dropped-response check, and a
  [process test](../../crates/urollup/tests/cli_process.rs) reports a Claude session
  beside an inconsistent record.
- [Pricing context](../../crates/urollup-core/tests/pricing_context.rs): recorded
  provider and tier per turn, complete snapshots and provider updates, which settings
  events set a child’s tier, conflicts that leave totals unchanged, and cache categories
  through cumulative counters.
- [Counter unit tests](../../crates/urollup-core/src/ledger/counters.rs): epoch changes,
  repeated totals and checked deltas.
- [ccusage parity ledger](../../tests/parity/ledger.toml): every fixture difference from
  pinned ccusage, with its cause and retirement condition.

Merge status and the open Codex correctness fixes, including `uro-r8si`, `uro-eh0d`,
`uro-p9ua` and `uro-3b12`, live in the
[release readiness record](../project/specs/active/plan-2026-09-16-first-release-publishing.md#open-correctness-fixes).
A seeded child whose migration kept only the suffix from its own compaction reports its
seed as an unverified gap even when no response was dropped (`uro-zq65`). Rates and
price matching are follow-up work under `uro-wuby` and `uro-neii`. Real-history
comparator residuals remain to be explained by `uro-d36a` and `uro-ky6c`; the synthetic
matrix is not full-history acceptance.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
