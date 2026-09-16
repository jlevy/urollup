# Dialect Fixtures

Frozen fixtures for the `claude-project` and `codex-rollout` adapters, one directory per
case, each a small discovery root plus an `expected.json` holding the reconciled truth
and the naive sums a reader gets without reconciliation.

**All fixture content is synthetic.** IDs, prompts, replies, tool inputs, paths,
branches and token counts were invented for these cases; nothing was copied from a local
agent log, and no home directory, account name, email address or credential appears in
the tree. Record shapes come from public source at pinned commits and from the project’s
research briefs. `scripts/check-fixtures.mjs` (`make fixtures-check`) enforces this:
every line parses unless the case declares it malformed or pending, `.jsonl.zst` members
decompress, `expected.json` lists exactly the case’s files and adds up, and nothing
looks private.

## Layout

A case directory is the agent’s own root, so the same fixtures drive urollup and the
milestone 0.1 ccusage parity harness:

- **`claude-project/<case>/`** is a Claude Code config directory: point
  `CLAUDE_CONFIG_DIR` at it.
  It holds `projects/-Users-example-project/<session>.jsonl`, with subagents in
  `<session>/subagents/agent-<agent-id>.jsonl` beside `agent-<agent-id>.meta.json`, and
  `subagents/workflows/<workflow>/` where a case needs it.
- **`codex-rollout/<case>/`** is a `CODEX_HOME`. It holds
  `sessions/YYYY/MM/DD/rollout-<local-time>-<thread-id>.jsonl`, a `_<rollout-id>` revert
  file, a `.jsonl.zst` twin and flat `archived_sessions/` where a case needs them.
  File names use local time (UTC-7 in these cases) while record timestamps are UTC, so a
  reader that takes dates from names gets them wrong.

## `expected.json`

One object per case, `format: urollup-fixture-expected/v1`:

| Field | Content |
| --- | --- |
| `summary`, `rules`, `agent` | What the case tests, the design rules it pins, and the agent versions it models |
| `files[]` | Every data file with its line count and thread |
| `decode` | Lines the case declares `malformed` or as its `pending_tail`, as `path:line` |
| `threads[]` | Native thread key, kind, parent and relationship, and `own` (and where useful `descendants`) requests and tokens |
| `requests[]` | One row per reconciled request: native key, identity basis, ownership and candidates, model and model basis, timestamp, tokens, optional `model_usage`, the `selected` record and all `evidence` |
| `copies[]` | Records that carry usage but add none, with their kind and what they copy |
| `totals` | Request counts by ownership status, `unresolved`, and token totals |
| `diagnostics[]` | Expected diagnostics with their refs |
| `limit_observations[]` | Provider limit observations, one per limit window, with native values kept verbatim |
| `naive[]` | What a reader gets without reconciliation, one row per naive rule, so tests can show the overcount |
| `notes` | Reasoning, interpretations and open questions |

Tokens use urollup’s normalized categories: `uncached_input`, `cache_read`,
`cache_write`, `output` and `reasoning` (`null` when the dialect records none, as Claude
transcripts do). Claude `input_tokens` excludes cache reads and writes, so it is
`uncached_input` directly; Codex `input_tokens` includes `cached_input_tokens`, so
`uncached_input` is their difference.
Codex cases keep `cache_write_input_tokens` at 0, because whether it sits inside
`input_tokens` is unverified.
Codex rows also carry `total_tokens`, which is where counter overcounts show up.
Every `path:line` reference is relative to the case directory and 1-based.

Diagnostic codes are provisional names for the ledger to adopt or rename:
`claude-block-usage-conflict`, `claude-cache-creation-breakdown-mismatch`,
`claude-nested-copy-without-original`, `identity-key-conflict`,
`codex-counter-epoch-reset`, `codex-estimate-compaction`,
`codex-estimate-context-window-fill`, `codex-copied-history-inferred`, `thread-orphan`,
`codex-rollout-duplicate-location`, `malformed-line` and `pending-tail`.

## Cases

Expectations were derived by hand from the design rules named in each row; every case
README explains the reasoning and names the pinned source its shapes come from.

| Case | What it tests | Design rule | Shape source | Models |
| --- | --- | --- | --- | --- |
| `claude-project/brief-double-counting` | One response as three block records plus a resumed replay: the brief’s 4x overcount | §3.3, §3.4, §3.2 | Research brief; session-report analyzer | Claude Code 2.1.x |
| `claude-project/block-record-selection` | Block records disagreeing on `output_tokens` and on cache fields; selection order and the conflict diagnostic | §3.4 | receipts miner; ccusage duplicate test | Claude Code 2.1.x |
| `claude-project/progress-nested-subagent` | `progress` records nesting subagent messages, one with its subagent file and one without | §3.4, §3.3, §3.2 | ccusage progress fixture; agentfdr `.meta.json` | Claude Code 2.1.x |
| `claude-project/btw-side-question` | A `/btw` replay of a parent message under a new `requestId` | §3.4 | ccusage sidechain notes and tests | Claude Code 2.1.x |
| `claude-project/fork-subagent-uuid-replay` | A fork-style subagent replaying parent `uuid`s, one under a new `requestId` | §3.4 | session-report analyzer | Claude Code 2.1.x |
| `claude-project/advisor-iterations` | `iterations` with an `advisor_message` carrying its own model | §4.1 | ccusage advisor fixture | Claude Code 2.1.x |
| `claude-project/nested-null-tool-input` | Nested nulls in tool input that ccusage rejects | §2.2 | ccusage null filter and tests | Claude Code 2.1.x |
| `claude-project/timestamp-precision` | 0, 1, 2, 6 and 9 fractional digits and a non-UTC offset across a UTC day boundary | §2.2, §4.3 | ccusage timestamp parser | Claude Code 2.1.x |
| `claude-project/missing-request-id` | No `requestId` with Bedrock-style `msg_bdrk_` IDs | §3.6, §3.4 | ccusage requestless duplicate test | Claude Code 2.1.x |
| `claude-project/quota-limits` | `quotaLimits` entries and a `<synthetic>` usage-limit error record | §3.1, §4.4, §4.1 | Research brief; ccusage error parser | Claude Code 2.1.x |
| `claude-project/workflow-subagents` | `subagents/workflows/` transcripts and a nested spawn edge | §3.2, §4.2 | agentfdr workflow test; ccusage paths | Claude Code 2.1.x |
| `claude-project/cache-creation-breakdown` | A 5-minute and 1-hour breakdown disagreeing with the flat count | §4.1 | ccusage cache accessor; receipts weights | Claude Code 2.1.x |
| `claude-project/gateway-message-id-reuse` | One `message.id` reused across sessions, beside a true copy | §3.6, §9.1, §4.2, §3.2 | ccusage commit `a4b8420` | Claude Code 2.1.x |
| `codex-rollout/brief-repeated-snapshot` | Cumulative `token_count` with a repeated identical snapshot: the brief’s example | §3.4, §3.1 | Codex protocol and rollout tests | Codex CLI 0.150.0 |
| `codex-rollout/info-null` | `info: null` before the first usage, and a rate-limit-only update | §3.4, §3.1 | Codex `turn.rs` and session tests | Codex CLI 0.150.0 |
| `codex-rollout/counter-reset-epoch` | A cumulative total that restarts lower, opening a new epoch | §3.4, §3.3 | ccusage total-only path | Codex CLI 0.150.0 |
| `codex-rollout/compaction-and-context-full` | A compaction estimate and a context-window-full fill | §3.4, §4.1 | Codex estimates, fills and compaction test | Codex CLI 0.150.0 |
| `codex-rollout/token-usage-records` | `token_usage_record` across a resume, a `compacted` copy, and fork copies naming the parent thread | §3.4, §3.2 | Codex `TokenUsageRecord`, wire shapes, fork persistence | Codex CLI 0.154.0 |
| `codex-rollout/legacy-subagent-prefix` | Legacy subagent prefixes with and without a discovered parent, and a prefix without counters | §3.4, §3.2 | Codex spawn copies; ccusage replay tests; squares prefix cut | Codex CLI 0.150.0 |
| `codex-rollout/paginated-subagent` | A native boundary from `subagent_history_start_ordinal` | §3.4, §3.2 | Codex live thread and ordinal validation | Codex CLI 0.154.0 |
| `codex-rollout/legacy-user-fork-counters` | A legacy user fork copying `token_count` events, its counter continuing the parent’s | §3.4, §3.2 | Codex thread manager; ccusage copied-branch test | Codex CLI 0.150.0 |
| `codex-rollout/revert-file` | One thread over an original and a `_<rollout-id>` revert file with `history_base` | §2.1, §3.3, §3.6 | Codex revert, file names and ordinals | Codex CLI 0.154.0 |
| `codex-rollout/zst-twin` | A `.jsonl` and its `.jsonl.zst` twin as one logical source | §2.2, §3.3 | Codex compression worker and discovery | Codex CLI 0.154.0 |
| `codex-rollout/archived-rename` | Flat `archived_sessions/`, a compressed archived child, a leftover active copy, local-time names | §2.1, §3.6, §3.2 | Codex archive rename; ccusage archived discovery | Codex CLI 0.154.0 |
| `codex-rollout/multi-limit-id` | `rate_limits` alternating `limit_id`, with a repeat and carried-forward fields | §3.1, §4.4 | Codex rate-limit types and header parsing | Codex CLI 0.154.0 |
| `codex-rollout/auto-review-model` | The `codex-auto-review` placeholder model and a tier-less settings event | §3.1, §3.2 | ccusage placeholder handling; Codex guardian sessions | Codex CLI 0.154.0 |
| `codex-rollout/pending-tail` | Interior corruption and an unfinished last line | §2.2 | Codex line skipping and truncated-tail test | Codex CLI 0.154.0 |

Design sections are in [docs/urollup-design.md](../../../../docs/urollup-design.md), and
the double-counting example is in the
[portable research brief](../../../../docs/project/research/research-2026-09-13-portable-agent-usage.md#synthetic-double-counting-example).

## Not modeled yet

- **Other dialects:** `claude-stream`, `codex-exec` (including cumulative
  `turn.completed.usage` across a resume), `pi-session` and `pi-events`. Captured
  streams arrive in milestone 0.5 and Pi in Phase 2, with their own fixtures.
- **Migrated Codex rollouts:** `codex migrate-rollouts --apply` rewrites a legacy file
  in place as paginated, dropping rolled-back usage records.
  Modeling it needs a before and after pair plus capture-store expectations, which
  belong with the capture work.
- **Claude Code inline sidechains:** older transcripts that keep subagent turns inline
  with `isSidechain` and no spawn ID.
- **Cases derived from real sessions:** the maintainer approved sanitizing this
  project’s own Claude Code logs, but the running environment’s permission system
  refused access to `~/.claude`, so every Claude case here stays synthetic.

## Research notes, 2026-09-15

Shapes these fixtures needed that go beyond the research briefs, all read from Codex
`rust-v0.154.0` at
[`6b9826e`](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340):

- A revert file’s own records continue the ordinal sequence: a new paginated rollout
  starts at `history_base.end_ordinal_exclusive`, so its `session_meta` is not at
  ordinal 0
  ([ordinal.rs:16-53](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/ordinal.rs#L16-L53)).
- `HistoryPosition.thread_id` names a **rollout** ID, not the stable thread ID, and
  carries an exclusive end ordinal and the byte offset after the last included record
  ([protocol.rs:3018-3032](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3018-L3032)).
- The persisted turn events are `task_started` and `task_complete` on the wire, with
  `turn_started` only an alias, and `task_complete` carries terminal `error` details
  ([protocol.rs:1403-1419](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L1403-L1419),
  [2141-2164](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2141-L2164)).
- A second metered bucket appears as `limit_id: "codex_other"`, from the
  `x-codex-other-*` header family; snapshots also carry `limit_name` and
  `normal_model_slug`
  ([rate_limits.rs:23-100](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/codex-api/src/rate_limits.rs#L23-L100)).
- A guardian review’s session source serializes as
  `{"subagent": {"other": "guardian"}}`, beside `thread_source: "guardian_review"`
  ([protocol.rs:2815-2840](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L2815-L2840)).
- A truncated (not full-history) fork drops `turn_context` and `world_state` from the
  copied prefix, while a full-history fork keeps them
  ([spawn.rs:65-105](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105)).
- These rollouts carry only the fields an accounting reader needs; Codex’s own typed
  deserializers require more (a `thread_settings_applied` snapshot, for example, also
  has `approvals_reviewer`, `permission_profile` and `collaboration_mode`). That is
  deliberate: readers decode rollout lines through `serde_json::Value`, never a strict
  struct
  ([history/src/lib.rs:253-264](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/history/src/lib.rs#L253-L264)).

Two Claude shapes stay unverified and are flagged in their cases: the `quotaLimits`
object (the research brief records the keys `rateLimitType`, `resetsAt` and `status`,
not the object layout) and the `agent-aaside_question-<hex>` file name for `/btw` side
questions, which combines ccusage’s note that `/btw` logs live under `subagents/` with
session-report’s labeled-agent file convention.

Environment note: `.jsonl.zst` members were written with Node’s `zlib.zstdCompressSync`
at level 3, the level Codex compresses with.
Node gained zstd in 22.15 and 23.8; the fixture check skips decompression with a note on
older runtimes.

## Attribution

Cases adapted from other projects’ tests keep only the shape of the case: every ID,
path, prompt and token count is new, and the expected results are urollup’s, which
differ from the source’s where the source is wrong.

- **ccusage**
  ([`bd7f89b`](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1),
  MIT, Copyright (c) 2025 ryoppippi): the advisor-iteration, `progress`, null-field,
  sidechain-replay, requestless-duplicate, gateway-reuse, Codex replay and repeated
  snapshot cases, and the negative cases that record ccusage bugs.
- **OpenAI Codex**
  ([`6b9826e`](https://github.com/openai/codex/tree/6b9826e3aa83b1a5947db50f4332cb9c65f1b340),
  Apache-2.0, NOTICE: OpenAI Codex, Copyright 2025 OpenAI): rollout line and payload
  shapes, and the token scenarios (records across resume, `info: null`, context-full
  fills, compaction estimates, subagent non-inheritance).
- **Anthropic claude-plugins-official**
  ([`f0dce59`](https://github.com/anthropics/claude-plugins-official/tree/f0dce59fec064db10450cb6ed6e33c1080d61537),
  Apache-2.0): block-record and `uuid`-replay behavior from the session-report analyzer
  and the receipts miner.
- **agentfdr**
  ([`e0904bf`](https://github.com/kamihork/agentfdr/tree/e0904bf8791f90916fa8db2ce702df93a7caee90),
  MIT, Copyright (c) 2026 kamihork): Claude transcript and `.meta.json` key shapes.
- **squares**
  ([`f2e24e0`](https://github.com/jlevy/squares/tree/f2e24e07be8c94fa3ac603c3534dce7c454da99b),
  MIT, the maintainer’s own repository): the synthetic Codex record builder design and
  the legacy subagent prefix-cut case.

No source file was copied, so no third-party notice file is required yet; see
[PROVENANCE.md](../../../../PROVENANCE.md).

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
