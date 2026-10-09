---
title: "Cursor Dialect with Model and Provider Facets"
description: Plan for adding Cursor as a urollup agent with first-class model and provider facets, without folding the work into milestone 0.1 or the Phase 2 Pi and Gemini CLI slice.
author: Joshua Levy with LLM assistance
date: 2026-09-19
status: Active; this plan's Phase 0 format facts are locked from research-2026-09-19-cursor-agent-logs.md (uro-890b); Cursor admission awaits a recorded decision (uro-jfaw); implementation is not part of milestone 0.1
---
# Feature: Cursor Dialect with Model and Provider Facets

## Overview

Cursor is a multi-model coding-agent surface.
One session can call Opus, Grok, Fable, GPT, and other models, and those models come
from several vendors.
urollup must roll that usage up beside Claude Code and Codex without treating “Cursor”
as a single model.

Local Cursor files do not carry Claude-grade or Codex-grade per-request usage.
They support session lists, catalog and picker model identifiers, an inferred vendor,
and occasional session-level `costInCents`. They do not support token-level
reconciliation or cache identities from disk alone.
See
[research-2026-09-19-cursor-agent-logs.md](../../research/research-2026-09-19-cursor-agent-logs.md)
(`uro-890b`).

This plan owns that dialect work: opt-in discovery, identity, the usage that is actually
present, facets, and synthetic fixtures.
Cursor is a candidate agent, not yet admitted.
A confirmed design decision (`uro-jfaw`) must record its admission before adapter work
starts, together with this plan’s departures from the candidate policy: no
`ccusage cursor` parity case, opt-in rather than default discovery, and the usage gap.
This plan does not change milestone 0.1, the
[scalable-ingestion plan](plan-2026-09-16-scalable-ingestion.md), or ingest-capacity
code.

The [product plan](plan-2026-09-13-urollup-cli-and-web.md) still owns Phase 1 through
Phase 3. Its Phase 2 dialect slice remains Pi and Gemini CLI
([Decision 28](../../../urollup-design.md#decision-28-gemini-cli-planned-support)). The
candidate policy for further agents stays in
[§9.1 Additional Agent Adapters](../../../urollup-design.md#additional-agent-adapters);
this plan proposes admitting Cursor by maintainer use, the rule that admitted Gemini CLI
through Decision 28. Until that decision is confirmed, Cursor stays a §9.1 candidate.
This plan’s own Phase 0 and Phase 1 are stages of this plan, not product phases.

## Goals

- Read Cursor’s local stores through a named dialect, only after the user opts in with
  `UROLLUP_CURSOR_*`, `--source`, or a later explicit default.
  Today’s default discovery finds none of this
  ([§2.1](../../../urollup-design.md#21-dialects-and-discovery)).
- Attribute every counted composer as **agent** `cursor`, never as Claude Code or Codex,
  even when the inferred vendor is Anthropic, OpenAI, or Cursor itself.
- Record **model** from evidence attached to the usage being counted: the `usageData`
  key for a per-model session cost, or the bubble’s own `modelInfo.modelName` for bubble
  usage. Current session `selectedModels[]` and picker values describe selections, not
  historical usage. `--group-by model` leaves unattributed usage unknown.
  Map stored `default` to Auto.
  Leave Fable’s picker-to-catalog model mapping open until a stored Fable value is seen.
- Record **provider** as the inferred vendor from a versioned catalog-family table
  (`Basis::Inferred` plus a diagnostic): Grok and Composer → `cursor`; `claude-*` →
  `anthropic`; `gpt-*` / `o3-*` → `openai`; `gemini-*` → `google`; `kimi-*` →
  `moonshot`; `glm-*` → `zai`. Provider is a first-class request facet and a
  `--group-by provider` dimension.
  There is no provider column on disk.
  A stored `claude-fable-*` value maps to `anthropic` through the `claude-*` rule; only
  a Fable spelling without that prefix stays unknown until a fixture shows it.
- Dedup by `composerId` so a JSONL transcript and a `composerData` row are one session.
  Subagent, resume, and Best-of-N edges follow recorded fields
  ([§3.3](../../../urollup-design.md#33-reconciliation),
  [§3.4](../../../urollup-design.md#34-dialect-reconciliation-rules)).
- Count only recorded usage the brief treats as usage.
  Session `usageData.costInCents` is a source estimate when present.
  Bubble `tokenCount` is often zero and is not a complete per-request ledger.
  `usageData.amount` is not a token count.
  Cache is not stored.
  Missing tokens stay unknown; they are never inferred from transcript length, tool
  text, or code-attribution stores
  ([§4.1](../../../urollup-design.md#41-measure-contracts)).
- Ship synthetic fixtures, goldens, and result checks only.
  Real Cursor logs never enter the repository
  ([product-plan testing](plan-2026-09-13-urollup-cli-and-web.md#golden-and-end-to-end-result-checks)).

## Non-Goals

- Milestone 0.1, its ingest acceptance (owned by `uro-zrr0` in the scalable-ingestion
  plan), or any change to ingest-capacity or scalable-ingestion work (`uro-n1cp` and its
  children).
- Phase 2 Pi and Gemini CLI adapters, `serve`, capture-cache reads, or the account
  registry.
- Claude-grade or Codex-grade per-request token and cache reconciliation from local
  Cursor files.
- A JSONL-only adapter as the usage owner.
  JSONL on the surveyed Mac covered 76 of 1,022 composers.
- Adding `~/.cursor` or `state.vscdb` to default discovery until a later opt-in
  decision. The `~/.cursor` tree mixes session logs with extension code, worker logs, and
  private prompts.
- Assuming `~/.cursor/chats/` or `store.db`. Both were absent on the surveyed install.
- Planning an adapter on `agentKv` without a binary decoder.
- A generic SQLite input feature.
  [Decision 20](../../../urollup-design.md#decision-20-database-input-in-phase-3) defers
  database input, including named external adapters, to Phase 3. This plan’s named
  `cursor-state` reader of Cursor `state.vscdb` follows that ordering unless the
  exception proposed on `uro-2hck` is confirmed; that exception would cover this one
  reader, not generic SQLite input.
- Collapsing Cursor into one model bucket, or using `cursor` as a `--group-by model`
  value.
- Assigning provider `cursor` to every request.
  Cursor is always the agent.
  Provider is the inferred vendor, which is `cursor` only for Cursor-hosted families
  (Grok, Composer).
- Inferring token counts, list-price amounts, Auto’s served model, or account
  identifiers.
- Treating `usageData.amount` as tokens, or `contextTokensUsed` / `contextUsagePercent`
  as billed request usage.
- A `ccusage cursor` parity case.
  ccusage 20.0.20 does not read Cursor
  ([ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory)).
- Cloud-only Cursor usage that never reaches a local store, Cursor Tab or inline
  completion telemetry as a request ledger, MCP or plugin traffic, or AgentStores
  working files.
- Committing real Cursor transcripts, state databases, prompts, paths, identifiers, or
  values.

## Background

### Why a Separate Plan

Cursor is maintainer-used and multiplexes models in a way Claude Code and Codex do not.
Folding it into milestone 0.1 would delay the Claude and Codex alpha.
Folding it into Phase 2 would mix this dialect into the Pi, Gemini CLI, and web-UI
slice. The ingest engine is JSONL-oriented; the authoritative Cursor store is a
multi-gigabyte WAL-backed key-value database.
A focused plan matches [scalable ingestion](plan-2026-09-16-scalable-ingestion.md) and
[first-release publishing](plan-2026-09-16-first-release-publishing.md): one concern,
its own beads, a pointer from the product plan.

### How Claude Code and Codex Set Facets Today

The 0.1 engine already separates surface, format, vendor namespace, and model:

| Facet | Where it lives | Claude Code (`claude-project`) | Codex (`codex-rollout`) |
| --- | --- | --- | --- |
| Agent | `selection::Agent`, `QuerySource.agent`; summary `properties.agent` is designed ([§5.2](../../../urollup-design.md#52-usage-summary-format)), not implemented | `claude` | `codex` |
| Dialect | source / adapter | `claude-project` | `codex-rollout` |
| Model | `Request.model` (`served` or `requested`) | `message.model`, `ModelBasis::Served` | `turn_context.model`, `ModelBasis::Requested` |
| Provider (identity only) | request-key namespace | `anthropic` | `openai` |
| Account | `Thread.account` | `Basis::Unknown` | `Basis::Unknown` |
| Effort | `Request.effort` | recorded when present | `turn_context` effort when present |

`--group-by` in milestone 0.1 is `project`, `account`, `model`, and `effort`, on
`report` only: each dimension is a separate breakdown, and `daily` and `sessions` reject
the flag until joint calendar and dimension grouping (`uro-qvp1`) exists.
Model rows use the request model, or per-component `ModelUsage` when one request names
more than one model.
Account grouping currently yields `unknown` because neither adapter records a stable
account. The session index and current-session detection use the `Agent` enum; the
`--agent` flag is planned for milestone 0.5
([§10.4](../../../urollup-design.md#104-flag-index)) and is not in the 0.1 CLI.
`--group-by agent` is designed
([§10.6](../../../urollup-design.md#106-ccusage-use-case-coverage)) but not implemented
in the 0.1 `GroupBy` enum.

Provider is not a report dimension today.
It appears as the identity-key namespace (`IdScope::Provider`, tokens `anthropic` and
`openai`) and later as the price-table vendor
([§4.5](../../../urollup-design.md#45-price-table)). That collapse is harmless while
each agent has one vendor.
Cursor breaks it: agent `cursor` plus models from several vendors, with no provider
field in the files.

### Locked Format Facts

This plan’s Phase 0 (`uro-3fbc`, closed) locked these facts to
[research-2026-09-19-cursor-agent-logs.md](../../research/research-2026-09-19-cursor-agent-logs.md)
(`uro-890b`, closed).
Evidence is one maintainer Mac on **Cursor 3.21.13** (2026-09-19). The schema is
unofficial and version-dependent.
Field names and structural paths only; no prompts, paths, identifiers, or raw usage
values from that disk.

| Fact | Lock |
| --- | --- |
| **Authoritative store** | `~/Library/Application Support/Cursor/User/globalStorage/state.vscdb`, keys `composerData:<composerId>` and `bubbleId:<composerId>:<bubbleId>` in `cursorDiskKV`. About 4.2 GiB plus an 11 MiB WAL on the surveyed Mac (1,022 composers, 143,353 bubbles). Snapshot while Cursor is open. |
| **JSONL export** | `~/.cursor/projects/<workspace-slug>/agent-transcripts/<uuid>/<uuid>.jsonl`, with `subagents/` children. Thin: 76 of 1,022 composers. No tokens, cache, cost, timestamps, message IDs, or session-level model. Tool names are Cursor agent names. |
| **Absent here** | `~/.cursor/chats/` and `store.db`. Third-party maps still document that layer for Agents Window / CLI. |
| **Not a decoder target** | `agentKv` (197,010 keys; sampled rows are not useful JSON at this version). |
| **Not usage** | `conversation-search.db` (titles and bodies), workspace `state.vscdb` `composer.composerData` (UI pointers), AgentStores (mailbox / sync), `ai-code-tracking.db` (code-hash provenance), `agent-tools/*.txt` (tool dumps), `state.vscdb.backup`. |
| **Session id** | `composerId` UUID. JSONL folder name matches when a transcript exists. All 76 parent JSONL UUIDs had a `composerData` row. The brief’s separate count of 935 composers without JSONL is unreconciled; do not derive an exact missing-session total from these aggregates. |
| **Turn / request ids** | Bubbles list on `fullConversationHeadersOnly`. Some bubbles carry `requestId` and `usageUuid`. JSONL has `{role, message}` and `turn_ended` lines, no request id. |
| **Modes** | `unifiedMode` / `isAgentic`: `agent`, `chat`, and rare `multitask` / `plan` / `background`. |
| **Subagents** | JSONL `subagents/`, `composerHeaders.isSubagent`, parent `subComposerIds`. |
| **Usage** | JSONL: none. `usageData` non-empty on 135 of 1,021 composers; payload is `costInCents` plus `amount` (not tokens). Bubble `tokenCount.inputTokens` / `outputTokens` present on sampled bubbles, nonzero on 297 of 8,000, assistant-only. No cache-create or cache-read fields. Context-window snapshots (`contextTokensUsed`, `contextTokenLimit`, `contextUsagePercent`, `promptTokenBreakdown`) are occupancy, not billed usage. SDK `run.usage` / `agent.getUsage()` were not found as complete local records. |
| **Model** | Picker / effort label: `modelConfig.modelName`, `bubble.modelInfo.modelName`, `usageData` keys (e.g. `claude-4.5-opus-high-thinking`, `cursor-grok-4.6-xhigh-fast`). Catalog id: `selectedModels[].modelId` (e.g. `claude-opus-4-5`, `grok-4.6`). Neither is a provider wire id (no dated Anthropic snapshots, no `xai/…`). Auto is the literal `default`. No `claude-fable-*` on this disk. |
| **Provider** | Not stored. Infer from the catalog-family table above, or Cursor’s [Models & Pricing](https://cursor.com/docs/models) provider column. |
| **Default discovery** | urollup default roots are `~/.claude/projects` (and XDG `claude/projects`) and `~/.codex`. A default `urollup report` on the surveyed machine includes zero Cursor sessions. |
| **Cloud** | `conversation-search` `cloud-cache` rows and `bc-` keys in `ItemTable`. No local cloud-transcript JSONL tree. `composerHeaders` had no `bc-` ids. |
| **Accounts** | One `User/` profile. No second Cursor login root on this disk. |
| **First fixtures** | Claim Cursor 3.21.13 shapes. Re-check `store.db` and Fable on a later version or a second install. |

**Risks the adapter must treat as constraints:**

- Unofficial, version-dependent schema
- WAL and a multi-gigabyte live database
- JSONL-only ingest misses most sessions
- UUID join of JSONL and `composerData` double-counts unless `composerId` wins
- Auto (`default`) hides the served model

## Design

### Facet Contract

Keep Cursor inside the existing Agent / dialect / request model.
Add only what Cursor forces: a vendor facet that is no longer implied by the agent, and
honest coverage for missing tokens.

| Facet | Cursor rule | Justification |
| --- | --- | --- |
| **Agent** | `cursor` | Cursor is the coding-agent surface, the same role as `claude` and `codex`. Selection (`--agent cursor`) and `QuerySource.agent` use this token, as will summary `properties.agent` once the designed summary artifact ([§5.2](../../../urollup-design.md#52-usage-summary-format)) exists. |
| **Dialect** | Registry token `cursor-state` for the state-store owner (`composerData` / `bubbleId`), with JSONL as the same session when the folder UUID matches. Do not emit a second session from JSONL. A `store.db` dialect waits until that tree is seen. | A dialect is one format written by one agent ([§2.1](../../../urollup-design.md#21-dialects-and-discovery)). The brief names the stores, not a urollup token; this plan’s Phase 1 picks `cursor-state`. |
| **Model** | Use the model field attached to the measured record: `usageData` key for session/model cost, bubble `modelInfo.modelName` for bubble tokens. Preserve current session `selectedModels[]` and picker `modelName` separately as selection metadata. Do not use them to relabel historical usage. Map `default` to Auto; absent historical attribution stays unknown. | `--group-by model` must split catalog families. The token `cursor` is never a model value. Claude and Codex record a provider model id; Cursor records Cursor’s own catalog and picker spellings. |
| **Provider** | Inferred vendor from the catalog-family table, `Basis::Inferred`, with a diagnostic. Never invent a column. Auto and unmapped families stay unknown. A `claude-fable-*` value maps to `anthropic` through the `claude-*` rule; a Fable spelling without that prefix stays unknown until seen. | This is the existing identity and price-table meaning of provider. Cursor is the first agent that multiplexes vendors, so provider becomes a request field and a `--group-by` dimension rather than an adapter constant. |
| **Account** | Observed stable account identifier, else unknown | Same rule as Claude and Codex: never guess from model or subscription ([§2.1 Projects and Accounts](../../../urollup-design.md#projects-and-accounts)). |
| **Effort** | Recorded thinking or effort field, else omitted. On this install that field is integer `thinkingStyle` (observed 1 and 2), stored as `style-{n}`. Picker labels often encode effort (`-high-thinking`, `-xhigh-fast`); do not parse those suffixes into `Request.effort`. | Same as `Request.effort` today. A later named level can replace `style-{n}` if Cursor starts writing one. |

Historical model attribution follows the grain of each measurement:

- A `usageData` entry names the model for that session/model cost only.
  Do not apply its cost or model to individual bubbles without a recorded join.
- A bubble’s own `modelInfo.modelName` labels its recorded tokens.
  If absent, leave the model unknown.
  Public readers’ user-to-assistant propagation rule needs a versioned fixture and
  explicit evidence before this adapter adopts it.
- `modelConfig.selectedModels[]` may list multiple selections, and the composer’s picker
  may have changed. Neither supplies a historical fallback, even when the current list
  has only one item. Normalize picker and catalog spellings only through an explicit,
  versioned mapping; preserve the native label and attribution basis.
- Provider inference follows the model attributed to that measurement.
  An unknown historical model also leaves its provider unknown.

`--group-by provider` is required for this dialect, and `--group-by agent` is useful in
mixed Claude, Codex, and Cursor reports (promised in the
[coverage table](../../../urollup-design.md#106-ccusage-use-case-coverage)). The generic
agent and provider facets, with their versioned mappings and a separate billing channel,
belong to `uro-qvp1` in the usage-analysis workflow plan.
This plan contributes only the Cursor catalog-family mapping table and its fixtures, and
the Cursor adapter uses those generic facets.

Identity keys follow the brief’s uniqueness scope
([§3.6](../../../urollup-design.md#36-analytical-identities)):

- Vendor-issued response IDs, if any later appear, use `IdScope::Provider` and the
  inferred vendor token.
- Cursor-issued request IDs (`requestId`, `usageUuid`) use `IdScope::Thread` and the
  composer’s analytical thread ID. Live `state.vscdb` reuses the same `requestId` across
  composers; agent-scoped keys produced `conflicting-shared-key` and
  `unresolved-candidate` diagnostics.
  Composer or conversation IDs alone are not request keys.
- Thread keys use `composerId` under `agent_thread_identity(Agent::Cursor, …)`.

### Discovery Roots

Discovery is opt-in.
Do not add Cursor paths to the default root list until a later decision.

| Root class | Conventional locations | Role |
| --- | --- | --- |
| Application user state | macOS `~/Library/Application Support/Cursor/User/globalStorage/state.vscdb` (surveyed); Linux `$XDG_CONFIG_HOME/Cursor/User/globalStorage/` (default `~/.config/Cursor/…`) and Windows `%APPDATA%\Cursor\User\globalStorage\`, both conventional mappings that the brief did not verify | Usage owner: `composerData` and `bubbleId`. Requires a snapshot of the live SQLite file and WAL. |
| Project transcripts | `~/.cursor/projects/*/agent-transcripts/` | Same session as `composerId` when the folder UUID matches; evidence for tool-call text. Not a usage owner. Do not invent usage from these files. |
| Chat SQLite | `~/.cursor/chats/<workspace-hash>/<session-id>/store.db` | Absent on the surveyed Mac. Coverage gap until a later install confirms it; then a join or a second dialect, still keyed by the same UUID. |
| Workspace state | sibling `User/workspaceStorage/*/state.vscdb` | UI pointers (`composer.composerData`). Not a second transcript. |

Honor a native Cursor home variable if a later brief documents one.
The urollup override is a path list, `UROLLUP_CURSOR_DIRS` unless the dialect-ID
candidate in [§9.1](../../../urollup-design.md#dialect-ids-and-override-variables) picks
another name. `--source` and `--no-default-sources` work as for other agents.
A missing override or `--source` root exits 1. A machine with only default Claude and
Codex roots, and no Cursor opt-in, sees no Cursor rows.
What one override entry or `--source` path names for each root class, and what
JSONL-only input does, are open (`uro-tvc0`).

Do not discover even after opt-in:

- `AgentStores` (working files and sync indexes)
- `ai-tracking` (code attribution)
- `conversation-search.db` (search)
- `agentKv` (no decoder)
- extension, cache, GPU, cookie, and worker-log directories under the Cursor application
  tree or `~/.cursor/`
- `state.vscdb.backup`
- cloud or background-composer remotes that have no local replica

`sources` reports each opted-in root, dialect, and the earliest retained record the
brief can prove. Unsupported recognized formats are coverage gaps, not silence.

### Identity and Reconciliation Risks

Cursor sessions fork, resume, spawn subagents, run Best-of-N, and move between local and
cloud or worktree composers.
The adapter states copy and ownership rules from recorded fields, in the style of
[§3.4](../../../urollup-design.md#34-dialect-reconciliation-rules).

- **UUID join.** JSONL UUID equals `composerId` when both exist.
  `composerData` owns the session; JSONL must not add a second row.
  This is the double-count the brief measured (76 overlapping composers).
- **Subagent copies.** Transcripts under `subagents/` and headers with `isSubagent` or
  `subagentTypeName` may replay parent usage or hold their own calls.
  Spawn edges belong on `subComposerIds` or the JSONL parent folder; copied parent
  history does not count on the child.
- **Best-of-N.** `subComposerIds` and any `isBestOfNSubcomposer` field are sibling
  candidates. Only independently recorded usage counts; the others are copies or a
  documented coverage split, never a silent sum.
  Whether Cursor bills siblings separately is still open.
- **Resumes and forks.** A later composer that continues another conversation must not
  reclaim the original’s request IDs.
  Follow the Claude resume rule: a replay never wins ownership from discovery order.
- **Worktrees and cloud copies.** Workspace identifiers and background composers can
  duplicate the same `composerId` or `requestId` in two stores.
  Same analytical ID merges; conflicting payloads on one key are ambiguous
  ([§9.1 Conflicting Shared Keys](../../../urollup-design.md#conflicting-shared-keys)).
- **Drafts and archives.** `isDraft` and `isArchived` do not by themselves create or
  drop usage. Count recorded usage; do not skip an archived composer that still holds
  `usageData`.
- **Auto.** Stored name `default`. Do not invent the routed model.
- **Unobserved usage.** Tab completions, cloud runs without a local replica, SDK
  `run.usage`, and failed attempts the store never wrote are coverage gaps, never zero.

### Usage Fields Versus Inference

| Count | Do not count |
| --- | --- |
| `usageData.costInCents` when present, as a source estimate | `usageData.amount` as tokens |
| Bubble `tokenCount` only when the brief’s nonzero case applies, and never as Claude-style request completeness | Transcript text length, tool payloads, thinking duration, or character estimates |
| Context-window fields as coverage / occupancy diagnostics if a later report wants them | `contextTokensUsed` / `contextUsagePercent` / `promptTokenBreakdown` as billed request usage |
| Catalog / picker model strings and the inferred vendor | A provider guessed from prose, or Auto’s hidden served model |
| Observed account identifiers | Account inferred from model, plan UI, or email |

A source-reported cost is a source estimate, not a provider charge
([§3.1](../../../urollup-design.md#31-entities)). A present `tokenCount` of zero is not
a trustworthy usage record; it is the common case on this disk, not an observed
zero-token request. An absent `usageData` object is not a request ledger.
Token-level reconciliation against Claude or Codex identities is not supportable from
these files.

Strip policies follow
[§2.4](../../../urollup-design.md#24-capture-and-export-strip-policies): stub prompts,
assistant text, thinking text, tool arguments and results, attachments, and diffs; keep
types, IDs, timestamps, model fields, usage objects, and limit fields.
Unknown keys stay verbatim in capture and become stubs on export.

### Current Session

The `cursor-state` layer reads `CURSOR_CONVERSATION_ID` as `composerId` and maps it onto
`CurrentEnvironment` and `Agent::Cursor`. `--agent cursor` disambiguates when Claude or
Codex signals are also set.
If Cursor was not ingested, the CLI drops the signal so a default `report` without
opt-in stays `CurrentNotDetected`. The research brief does not document this variable,
so it is not yet the exact signal this plan requires; `uro-w5oa` stays open until a
consented check confirms it.
Cursor hooks stay rejected, like Pi’s. Hooks (`stop`, `preToolUse`, `postToolUse`,
`afterAgentResponse`) are a capture path, not a historical store; public notes say a
`stop` payload may include `conversation_id` and `transcript_path` and still lacks token
usage. `--latest` stays the only heuristic and is never implicit
([§6.2](../../../urollup-design.md#62-current-session-detection)).

### CLI and Reports

`report`, `daily`, and `sessions` include Cursor when an opt-in root, override, or
`--source` finds it.
`--agent cursor` selects only Cursor threads.
`report --group-by model` and `--group-by provider` give separate breakdowns for Cursor
composers, and `--group-by account` and `--group-by effort` use recorded values or
`unknown`. `daily` and `sessions` take no `--group-by` until `uro-qvp1` adds joint
grouping. Mixed-agent `--all` reports keep Claude, Codex, and opted-in Cursor in one
ledger, with one breakdown per requested dimension.
Default `--all` without a Cursor opt-in stays Claude and Codex only.

Pricing, when milestone 0.4’s table exists, matches on provider, billing channel and
model ([§4.5](../../../urollup-design.md#45-price-table)). Cursor rows carry billing
channel `cursor`, not the first-party API channel: Cursor bills its own hosted models
and charges third-party models at API rates
([research brief](../../research/research-2026-09-19-cursor-agent-logs.md)). A
first-party API list price applied to a Cursor row is therefore an estimate for that
channel.
The price matcher takes the native picker or catalog label through the versioned
label mapping in the Facet Contract (for example `claude-4.5-opus-high-thinking` →
`claude-opus-4-5`) and keeps the native label.
Unpriced Cursor models are pricing-coverage diagnostics, not invented rates.
Cursor-hosted Grok and Composer rows use provider `cursor`.

## Implementation Plan

This plan has two phases of its own, separate from the product plan’s Phases 1 to 3:
Phase 0 locks format facts, and Phase 1 builds the adapter.
There is no Phase 2 in this plan.

### Phase 0: Lock Format Facts

- [x] Accept
  [`research-2026-09-19-cursor-agent-logs.md`](../../research/research-2026-09-19-cursor-agent-logs.md)
  (`uro-890b`).
- [x] Lock the usage owner: `composerData` / `bubbleId` in `state.vscdb`; JSONL is a
  thin export of a subset of `agent` composers.
- [x] Lock model and vendor: catalog / picker ids on disk; provider inferred; no
  provider column.
- [x] Record the first-fixture Cursor version: 3.21.13 on the surveyed Mac.

Phase 0 is complete (`uro-3fbc` is closed).
Adapter implementation waits for the product’s Phase 3 database-input work under
Decision 20, or a confirmed exception to it, and for the questions below.
Research completion alone does not authorize an earlier database adapter.

### Open Questions Before Implementation

Each of these blocks the adapter bead `uro-p9ay`:

- **Admission decision (`uro-jfaw`).** A confirmed design decision must admit Cursor and
  record its departures from the
  [§9.1](../../../urollup-design.md#additional-agent-adapters) candidate policy: no
  `ccusage cursor` parity case, opt-in discovery, and the usage gap.
- **Snapshot and evidence (`uro-9m75`).** How a live WAL-backed store is snapshotted,
  what that costs on a multi-GiB file, what the manifest records, how a row is cited as
  evidence when the design’s evidence references are byte extents, and how decode stays
  within the per-agent row ceiling that the other adapters charge while decoding, and
  within the process-wide admission planned on `uro-6pi8`.
- **Usage mapping (`uro-jrir`).** For session `usageData.costInCents`: ledger entity,
  call count, timestamp and money measure.
  For bubble `tokenCount`: field semantics.
  For both: cache categories.
- **Request identity (`uro-qy0e`).** Which of `requestId` and `usageUuid` keys a
  request, and how bubbles that share a key combine their tokens.
- **SQLite reader (`uro-kddq`).** The proposed reader, `rusqlite` with bundled SQLite,
  reviewed under [SUPPLY-CHAIN-SECURITY.md](../../../../SUPPLY-CHAIN-SECURITY.md),
  recorded with the Decision 20 exception tracked on `uro-2hck`, and reconciled with the
  product’s Phase 3 storage choice.
- **Opt-in roots (`uro-tvc0`).** What an override entry or `--source` path names for
  each root class, and what JSONL-only input does.

### Phase 1: Facets, Adapter, and Fixtures

- [x] Add `Agent::Cursor` (`cursor`) through selection, discovery, CLI `--agent`, and
  report `QuerySource` wiring, beside `Claude`, `Codex`, and the existing Pi diagnostic
  variant.
- [ ] Use the generic request-level provider and agent facets, `--group-by provider` and
  `--group-by agent` from `uro-qvp1`, and add the Cursor catalog-family mapping
  (`Basis::Inferred` plus a registry token) with its fixtures.
- [ ] Implement opt-in discovery, the override, a WAL-safe snapshot of `state.vscdb`,
  snapshot manifests, and content-based identification of Cursor sources under
  `--source`, as settled in `uro-9m75` and `uro-tvc0`.
- [ ] Implement the state-store adapter: decode `composerData` / `bubbleId`, strip,
  `composerId` dedup, JSONL as the same session, subagent edges, coverage gaps for
  missing tokens and Auto, and provider limit observations only when the store records
  them.
- [x] Add synthetic fixtures under `crates/urollup-core/tests/fixtures/<dialect>/`,
  goldens under `tests/golden/e2e/<dialect>/`, and result checks.
  Derive fixture shapes from the brief or a structure-only sanitizer; never copy a real
  Cursor file into the tree.
- [ ] Establish an exact Cursor signal before implementing detection.
  If detection precedes selection support, add and test the exit-2 unsupported-dialect
  diagnostic. Without an exact signal, document that Cursor current-session detection is
  absent.
- [ ] Carry the confirmed admission decision (`uro-jfaw`) into
  [`docs/urollup-design.md`](../../../urollup-design.md) §2.1, §3.4, and §10.2, and keep
  the one-line pointers in the product plan current.
- [ ] Run a consented local verification that prints aggregates only, under the same
  privacy sentinels as `make e2e-local`. Do not commit that corpus.

### Draft Implementation Status

The `cursor-state` layer implements most of Phase 1 ahead of the decisions above, so its
items stay unchecked until those decisions are confirmed and the layer passes review.
It answers the questions before implementation provisionally:

- **Facets:** `GroupBy::Agent`, `GroupBy::Provider` and `GroupBy::Purpose` are separate
  `report` breakdowns, and the catalog-family table is `ledger/provider.rs`, ahead of
  the generic facets in `uro-qvp1`. `--group-by purpose` slices Cursor `unifiedMode`
  (`agent`, `chat`, `plan`, `multitask`, `background`). Composer `maxMode` is current
  picker state, not a historical usage facet.
- **Snapshot and evidence (`uro-9m75`):** `state.vscdb` is opened read-only with
  `query_only`, without a copy.
  Each store gets one manifest entry fingerprinted from its first 256 bytes, and
  evidence references count composer and bubble rows rather than bytes.
  Decode loads the selected composers and bubbles before reconciliation checks the
  observation ceiling; it does not charge admission while decoding.
- **Usage mapping (`uro-jrir`):** a bubble counts only when `tokenCount` input or output
  is nonzero, as uncached input and output; cache categories stay unknown.
  Session `costInCents` is a `cursor-estimate-cost` diagnostic, not a ledger amount.
- **Request identity (`uro-qy0e`):** `requestId` and `usageUuid` are thread-scoped keys,
  and a bubble with neither keys on its `bubbleId`. Bubbles that share a key keep the
  last one in store order under the latest-revision rule rather than summing.
- **SQLite reader (`uro-kddq`):** `rusqlite` with bundled SQLite, without default
  features.
- **Opt-in roots (`uro-tvc0`):** a `UROLLUP_CURSOR_DIRS` entry or `--source` path names
  a `state.vscdb` file, a `cursor-state.json` fixture, or a directory holding either.
  A Cursor JSONL transcript passed alone exits 1 as not a usage owner.
- **Current session (`uro-w5oa`):** `CURSOR_CONVERSATION_ID`, not yet verified (see
  [Current Session](#current-session)).

| Concern | Location |
| --- | --- |
| Opt-in roots | `DiscoveryEnvironment::cursor_roots` in `adapters/discovery.rs`; `UROLLUP_CURSOR_DIRS` |
| Dialect token and store open | `cursor_state::is_cursor_source`, `ingest_roots`, `read_sqlite` (`query_only`) |
| Composer / bubble decode | `load_composers`, `load_bubbles`, `attributed_model_name` |
| `--current` / `--session` family | `cursor_composer_ids` in `cli.rs`; `retain_composer_family` plus bubble-key prefix reads |
| JSONL path selector | `cursor_composer_id_from_selector` in `selection.rs`; UUID from transcript path, not usage |
| Project facet | `Thread.project` from `workspaceId` on `composerData` or `composerHeaders` |
| Thread / request keys | `agent_thread_identity(Agent::Cursor, composerId)`; `REQUEST_KEY` / `USAGE_KEY` (`IdScope::Thread`) |
| Cost estimate | `DiagnosticCode::CursorEstimateCost` in `ledger/diagnostics.rs` |
| Provider facet | `provider_for` / `infer_catalog_provider` in `ledger/provider.rs`; `GroupBy::Provider` |
| Purpose / effort | `Thread.purpose` from `unifiedMode`; `Request.effort` from `thinkingStyle` |
| Agent / current session | `Agent::Cursor` and `CURSOR_CONVERSATION_ID` in `selection.rs` |
| CLI third corpus leg | `Corpus::discover` in `cli.rs`; `classify_explicit_source` prefers Cursor |
| Reports | `query/aggregate.rs` `breakdowns`, `purpose_for_owners`, `provider_label` |
| Fixtures / goldens | `tests/fixtures/cursor-state/{basic,facets,subagents,best-of-n}/`; `tests/golden/e2e/cursor-state/{basic,facets,subagents,best-of-n}.tryscript.md` |

## Testing Strategy

- **Unit tests** cover opt-in discovery precedence, facet mapping (agent, catalog /
  native model, inferred provider, effort, account), `composerId` dedup, and zero-heavy
  `tokenCount`. Model cases include a changed current picker, multiple selected models,
  disagreement between bubble and session metadata, and missing bubble model; historical
  usage must keep its own attribution or remain unknown.
- **Goldens and result checks** follow
  [tests/golden/README.md](../../../../tests/golden/README.md): hermetic `HOME`, empty
  roots for every other agent, canaries in the opt-in Cursor locations the adapter
  honors, and `node scripts/new-e2e-golden.mjs <dialect>/<case>` for each new case.
- **No ccusage case** until ccusage grows a Cursor path past the cool-off; then a later
  bead can add one.
- **Local verification** is opt-in, aggregate-only, and sentinel-tested.
  It never writes identifiers or prompt text into docs or CI logs.

## Rollout Plan

Implementation belongs no earlier than the product’s Phase 3 database-input work, and
only after the questions before implementation are settled.
A confirmed exception to Decision 20, proposed on `uro-2hck`, is required to change that
ordering. It must not land in the 0.1.0 alpha
([first-release publishing](plan-2026-09-16-first-release-publishing.md)). This plan
does not change ingest acceptance, which `uro-zrr0` and the
[scalable-ingestion plan](plan-2026-09-16-scalable-ingestion.md) own, or other product
acceptance dependencies.

Ship behind opt-in discovery: users without a Cursor override or `--source` see no
change, including users who have a 4 GiB `state.vscdb`. `--agent cursor` with no
opted-in Cursor roots is an empty selection, not an error, matching other agents’
missing default roots.

## Open Questions

The questions that block implementation are under
[Open Questions Before Implementation](#open-questions-before-implementation).
These remain open without blocking it:

- Are Best-of-N siblings separately billed, or is one winner the sole counted usage?
- Does a later install grow `~/.cursor/chats/**/store.db`, and how does it join
  `composerId`?
- Which Cursor releases after 3.21.13 change these shapes, and how should the adapter
  version that claim?
- Should bubble tokens without `modelInfo` stay unknown, as the Facet Contract says, or
  fall back to the composer’s current `modelConfig.modelName`? The `cursor-state` layer
  uses the bubble value, then that composer fallback, because many bubbles omit
  `modelInfo`; one of the two must change before that layer is ready.
- Is `CURSOR_CONVERSATION_ID` the exact `--current` signal on every Cursor surface?

## References

- [Cursor agent-log research brief](../../research/research-2026-09-19-cursor-agent-logs.md)
  (`uro-890b`)
- [urollup design](../../../urollup-design.md), especially §2.1, §3.1, §3.4, §3.6, §4.1,
  §4.3, §4.5, §6.2, §9.1 Additional Agent Adapters, Decision 20, and Decision 28
- [Product plan](plan-2026-09-13-urollup-cli-and-web.md)
- [Portable agent-usage research](../../research/research-2026-09-13-portable-agent-usage.md)
- Golden and fixture rules: [tests/golden/README.md](../../../../tests/golden/README.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
