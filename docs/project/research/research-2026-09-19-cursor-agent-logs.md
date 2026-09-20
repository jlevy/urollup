---
title: Cursor Local Agent and Composer Session Logs
description: How Cursor stores agent and composer sessions on macOS, whether usage is recorded, and how model versus provider identifiers appear, so a plan spec can decide whether a Cursor dialect is feasible.
date: 2026-09-19
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete for one maintainer Mac on Cursor 3.21.13; schema is unofficial and version-dependent
---
# Research: Cursor Local Agent and Composer Session Logs

**Date:** 2026-09-19 (last updated 2026-09-19)

**Author:** Joshua Levy (github.com/jlevy) with LLM assistance

**Status:** Complete for the install below; not a Cursor storage specification

**Bead:** `uro-890b`

## Overview

urollup already reads Claude Code (`claude-project`) and Codex (`codex-rollout`)
persistent logs, where each request usually carries token counts, a model string, and
enough native IDs to reconcile once.
Cursor exposes many models in one picker (Auto, Composer, Grok, Opus, Fable, GPT, Gemini
and others) and bills some of them as first-party Cursor models and others at
third-party API rates.
A Cursor dialect is only worth planning if local files identify a session, a turn, and a
model, and if usage is recorded rather than inferred.

This brief inventories Cursor’s local stores on one maintainer Mac running **Cursor
3.21.13** on 2026-09-19. It quotes field *names*, record *shapes*, model-name *patterns*
and aggregate counts.
It does not copy prompts, project-identifying paths, session IDs or raw numeric values.

Public third-party readers describe a three-layer stack: JSONL under
`~/.cursor/projects/`, per-chat `store.db` under `~/.cursor/chats/`, and composer blobs
in `state.vscdb`. This install has the first and third layers and **no**
`~/.cursor/chats/` directory.
urollup’s default discovery would see none of them.

## Questions to Answer

1. Where does Cursor store agent and composer sessions on macOS, and what is the
   approximate volume?
2. How is a session identified, and how are turns or requests stored?
3. Is usage (tokens, cache, cost) recorded, or only inferred?
4. How do **model** and **provider** appear: Cursor marketing name, underlying provider
   model id, both, or neither?
5. Which dialects exist (composer chat vs agent vs cloud vs local; multiple accounts),
   and what would today’s urollup discover by default?
6. What is new relative to `claude-project` and `codex-rollout`?

## Scope

Included:

- Local files under `~/.cursor/` and `~/Library/Application Support/Cursor/` on this Mac
- Field names and shapes in agent-transcript JSONL, `state.vscdb`,
  `conversation-search.db`, `ai-code-tracking.db`, AgentStores and workspace
  `state.vscdb`
- Official Cursor model and pricing docs as of 2026-09-19
- Public third-party maps of Cursor storage (cited below)

Excluded:

- Implementing an adapter
- Uploading or exporting transcripts
- Reconstructing a bill from this install’s numbers
- Cursor Tab completions as a usage source
- Official confirmation of schemas (Cursor does not publish them)

## Findings

### Local Store Map

Cursor does not have a single transcript directory analogous to `~/.claude/projects/` or
`$CODEX_HOME/sessions/`. Session-related state on this Mac sits in overlapping layers:

| Layer | Generic location | Role on this install |
| --- | --- | --- |
| Agent transcripts | `~/.cursor/projects/<workspace-slug>/agent-transcripts/<session-uuid>/<session-uuid>.jsonl` | Readable agent turns; thin |
| Subagent transcripts | `…/agent-transcripts/<parent-uuid>/subagents/<subagent-uuid>.jsonl` | Same JSONL shape as the parent |
| Tool sidecars | `~/.cursor/projects/<workspace-slug>/agent-tools/*.txt` | Tool output dumps, not accounting |
| Composer database | `~/Library/Application Support/Cursor/User/globalStorage/state.vscdb` | Authoritative composer/agent state |
| Conversation search | `…/globalStorage/conversation-search.db` | FTS index of titles and bodies |
| Workspace leftovers | `…/User/workspaceStorage/<hash>/state.vscdb` | UI keys such as `composer.composerData` |
| AgentStores | `…/Cursor/AgentStores/cursor_agent_stores/<uuid>/` | Per-agent file store and sync mailbox |
| AI attribution | `~/.cursor/ai-tracking/ai-code-tracking.db` | Code-hash provenance, including a `model` column |
| Chat SQLite | `~/.cursor/chats/<workspace-hash>/<session-id>/store.db` | **Absent** here; documented by third parties for Agents Window / CLI |

Not session logs, and ignored for accounting: `~/.cursor/extensions/` (about 1.6 GiB),
editor `History`, Chromium `Cache`, `logs/`, and `anysphere.cursor-agent-worker` text
logs (about 1.6 GiB).

A project’s `.cursor/` directory holds rules, skills and editor config.
It is not a session log root.

### Volume on This Install

Cursor 3.21.13, one macOS user profile, 2026-09-19.

| Store | Files or rows | Bytes (approx.) |
| --- | --- | --- |
| `~/.cursor/` whole tree | 47k files | 1.9 GiB, mostly extensions |
| `~/.cursor/projects/` | 74 workspace slugs | 297 MiB |
| Agent-transcript JSONL | 265 files (76 parent, 189 subagent) | 22 MiB; 19,126 lines, all parseable |
| Agent-tools sidecars | 481 files | counted inside the projects tree |
| `state.vscdb` | 364,093 `cursorDiskKV` rows | 4.2 GiB, plus an 11 MiB WAL |
| `composerData:` keys | 1,022 | inside `state.vscdb` |
| `bubbleId:` keys | 143,353 | inside `state.vscdb` |
| `agentKv:` keys | 197,010 | inside `state.vscdb` |
| `composerHeaders` | 519 rows | table in `state.vscdb` |
| `conversation-search.db` | 798 conversations (784 `local`, 14 `cloud-cache`) | 16 MiB |
| workspace `state.vscdb` | 252 databases | 269 MiB |
| AgentStores | 200 dirs (199 UUID, 1 numeric `u*` personal store) | 678 MiB |
| `ai-code-tracking.db` | 27,708 `ai_code_hashes` rows | 15 MiB |
| `~/.cursor/chats/` | — | missing |
| `~/.cursor/prompt_history.json` | — | missing |

`state.vscdb` also has a 1.4 GiB `state.vscdb.backup` dated 2026-04-22, which is a
recovery artifact rather than a live dialect.

### How a Session Is Identified

Cursor uses more than one ID space.

- **Composer / agent conversation.** `composerData:<composerId>` in `cursorDiskKV`.
  `composerId` is a UUID. `composerHeaders.composerId` indexes a subset (519 of 1,022
  rows) and adds `workspaceId`, `createdAt`, `lastUpdatedAt`, `isArchived`,
  `isSubagent`, `subagentTypeName`, `recency` and a JSON `value`.
- **Transcript folder.** When an agent JSONL exists, its directory name is the same UUID
  as `composerId`. All 76 parent transcript UUIDs on this disk also have a
  `composerData:` row.
  A separate survey aggregate reported 935 composers without JSONL. These counts do not
  reconcile: 1,022 minus 76 is 946. The existing evidence does not explain whether the
  queries used different populations or joins; no overlap explanation is established.
  Treat the exact missing-session count as unverified.
- **Bubble.** One UI message or tool step is `bubbleId:<composerId>:<bubbleId>`.
  `composerData.fullConversationHeadersOnly[]` lists `{bubbleId, type, createdAt}` in
  order. Bubble `type` 1 is user; type 2 is assistant-side (text, thinking or tool).
- **Request.** Many bubbles carry `requestId`. 904 of 8,000 sampled bubbles had both
  `requestId` and `modelInfo`. 5,537 of those 8,000 had `usageUuid`. These IDs are
  request-scoped, not the session id.
- **Cloud.** `conversation-search` marks 14 rows `source=cloud-cache`. `ItemTable` has
  `glass/…` keys whose names include a `bc-` prefix, matching the Cursor SDK’s cloud
  agent id pattern (`bc-<uuid>`). `composerHeaders` on this disk had no `bc-` ids.
- **CLI / Agents Window `store.db`.** Third parties key those sessions by the same UUID
  as the JSONL folder.
  That tree does not exist here, so this brief cannot confirm the join on this machine.

A session in the IDE sense is therefore a **composer id**, optionally attached to a
JSONL file and optionally listed in `composerHeaders`. It is not one append-only file
the way `claude-project` and `codex-rollout` are.

`unifiedMode` on `composerData` (1,021 rows with the field) is the closest local dialect
tag:

| `unifiedMode` | Count |
| --- | --- |
| `agent` | 935 |
| `chat` | 69 |
| `multitask` | 15 |
| `plan` | 1 |
| `background` | 1 |

`isAgentic` is true on 935 composers and false on 86, matching `agent` versus the other
modes. `status` is `completed` (618), `none` (302) or `aborted` (101).

Subagents appear three ways: 189 JSONL files under `subagents/`, 196
`composerHeaders.isSubagent` rows (`generalPurpose`, `shell`, `explore`, `opus-xhigh`,
`ci-investigator`), and 51 parent composers with `subComposerIds`. JSONL `Task` tool
inputs sometimes set `input.model` to `inherit` (47) or `cursor-grok-4.6-xhigh-fast`
(6).

### How Turns Are Stored

**JSONL (agent transcripts).** One JSON object per line.
Parent files are nested (`<uuid>/<uuid>.jsonl`); none were flat
(`agent-transcripts/<uuid>.jsonl`) on this disk.

Observed top-level keys:

- `{role, message}` for 18,459 lines (`user` 1,717, `assistant` 16,742)
- `{type, status, error}` for 667 `turn_ended` lines (`success` 585, `error` 40,
  `aborted` 34)

`message` has only `content`, an array of blocks.
Block `type` is `tool_use` (39,690) or `text` (6,381). There is no `tool_result` block,
no per-line timestamp, no message id, no `model` and no `usage`. Tool names in
`message.content[].name` are the Cursor agent names (`Read`, `Shell`, `Grep`,
`StrReplace`, `Task`, …), not Claude Code names.

Cursor’s own forum states that JSONL omits tool outputs by design.
`agent-tools/*.txt` sidecars hold some of those outputs; they are not a usage ledger.

**`composerData` + `bubbleId` (authoritative for IDE history).** `composerData` holds
session metadata (`composerId`, `createdAt`, `unifiedMode`, `modelConfig`, `usageData`,
`fullConversationHeadersOnly`, `subComposerIds`, `contextTokensUsed`,
`contextTokenLimit`, `contextUsagePercent`, `promptTokenBreakdown`, line-edit totals).
Each bubble holds `text`, optional `thinking`, optional `toolFormerData` (tool name,
params and result), `tokenCount`, optional `modelInfo`, `requestId`, `usageUuid` and
many context lists (`relevantFiles`, `recentlyViewedFiles`, …).

Third-party parsers treat `state.vscdb` as higher fidelity than JSONL when the same UUID
exists in both. That matches this disk: JSONL is a lossy export of a subset of `agent`
composers.

**`agentKv`.** 197,010 keys, the largest family.
A 3,000-row sample decoded as JSON objects only once (`role=system`). The rest is not
useful JSON at this version; do not plan an adapter on `agentKv` without a binary
decoder. Public write-ups still report `providerOptions.cursor.requestId` and sometimes
`providerOptions.cursor.modelName` in older or different encodings.

**`~/.cursor/chats/.../store.db`.** Documented as tables `meta` and `blobs`, with
`meta.lastUsedModel` and sibling `meta.json`. Zero `store.db` files exist here.

**AgentStores.** Every store directory has `files/` and `.sync/`. `.sync/` holds
`index.sqlite`. `files/` JSONL sampled from 40 stores (131 lines) is a notification
mailbox (`v`, `delivery_id`, `subscription_id`, `subscription_type`,
`enqueued_at_unix_ms`, `notification`, `payload`), not a transcript.
The numeric `u*` directory is a personal store, not a conversation.

**Workspace `composer.composerData`.** Sampled workspace DBs still have this key, with
`allComposers`, `selectedComposerId`, `selectedChatId` and migration flags.
That is UI state pointing at the global composer store, not a second transcript.

### Is Usage Recorded?

**Partly, and not in a shape urollup can treat as a provider usage record.**

JSONL records **no** tokens, cache or cost.

`composerData` records:

- **`usageData`.** Present on 1,021 composers, **non-empty on 135**. Each entry is keyed
  by a model-name string.
  The payload has only `costInCents` (int) and `amount` (int).
  `amount` values on this disk fall in 1–999 (most in 10–99), so they are not token
  counts. `costInCents` is usually 1–999, sometimes 1,000–9,999, and nonzero on 159 of
  160 payloads. There are no cache fields.
- **Context window snapshot.** `contextTokensUsed`, `contextTokenLimit` and
  `contextUsagePercent` on roughly 680–700 composers.
  `promptTokenBreakdown` (`totalUsedTokens`, `maxTokens`, `categories`) on 219. These
  describe prompt construction / context fill, not billed request usage.

Bubbles record:

- **`tokenCount.inputTokens` and `tokenCount.outputTokens` on every sampled bubble.**
  Nonzero on 297 of 8,000 samples, and only on assistant (`type=2`) bubbles.
  No cache-create or cache-read fields.
- **`usageUuid`** on 5,537 of 8,000, without an accompanying token object beyond
  `tokenCount`.

`ai_code_hashes` links a hash to `requestId`, `conversationId`, `source` (every row here
is `composer`) and `model`. That is provenance for edited code, not a usage ledger.

The Cursor SDK documents live `run.usage` / `result.usage` (`input`, `output`, `cache`,
`total`, optional reasoning) and a separate billed `agent.getUsage()` call.
Those objects are API/runtime surfaces.
They were not found as complete records in the IDE JSONL or in `usageData`.

Third-party readers disagree on coverage: vibe-replay reports `inputTokens` /
`outputTokens` on some bubbles and warns that aggregates are a lower bound; Watchmen and
CodeBurn report many Cursor v3 bubbles with token counts of zero and fall back to
character estimates.
This install matches the zero-heavy picture.

**Implication for urollup:** cost is sometimes stored at session/model grain
(`usageData.costInCents`). Per-request tokens are often missing.
Cache is not stored.
A Cursor adapter cannot satisfy the same reconciliation identities as `claude-project`
or `codex-rollout` from local files alone.

### How Model and Provider Appear

**Both a Cursor picker label and a catalog model id appear.
A provider field does not.**

Official pricing ([Models & Pricing](https://cursor.com/docs/models)) lists display
names with an explicit **Provider** column (`Cursor`, `Anthropic`, `OpenAI`, `Google`,
`SpaceXAI` as training partner, and others).
Cursor-hosted models are Grok 4.6 / 4.5 and Composer 2.5. Auto / Cursor Router bills at
the routed model’s list price; the routed identity may be hidden unless a team admin
sets visibility to Displayed.

On disk, five string families appear.

**1. `composerData.modelConfig.modelName` (1,021 of 1,022 composers).** This is the
picker / effort label currently stored on the composer.
Patterns on this disk:

| Pattern | Count | Notes |
| --- | --- | --- |
| `claude-4.5-opus-high-thinking` | 345 | marketing and effort, not `claude-opus-4-5` |
| `grok-4.6` | 220 | Cursor / xAI catalog id |
| `claude-4.6-opus-high-thinking` | 107 |  |
| `composer-1` | 41 | older Composer label |
| `gemini-3-pro` | 39 |  |
| `gpt-5.1-codex-high` | 33 | GPT family + effort |
| `gpt-5-high` | 29 |  |
| `gpt-5.2` | 29 |  |
| `claude-4.5-sonnet-thinking` | 28 |  |
| `cursor-grok-4.6-xhigh-fast` | 25 | Cursor-prefixed Grok variant |
| `gpt-5-pro` | 24 |  |
| `grok-code-fast-1` | 17 |  |
| `default` | 9 | Auto; Watchmen maps this to `auto` |
| `claude-opus-4-6` | 8 | closer to catalog id |
| `claude-opus-5` | 5 |  |
| `composer-2.5` / `composer-2.5-fast` | 2 | current Composer catalog ids |
| `kimi-k2-instruct` | 3 |  |
| `glm-5.2` | 1 |  |
| comma-joined multi-ids | 7 | rare; looks like concatenated selections |

Other `modelConfig` keys: `maxMode` (true on 874, false on 147) and `selectedModels`
(list on 392 composers).

**2. `modelConfig.selectedModels[].modelId`.** A list of `{modelId, parameters}`
objects. `parameters` was an empty object on every sampled row.
`modelId` patterns are closer to the public catalog than `modelName`:

| Pattern | Count |
| --- | --- |
| `grok-4.6` | 248 |
| `claude-opus-4-5` | 48 |
| `default` | 44 |
| `claude-opus-4-6` | 30 |
| `claude-sonnet-4-5` | 13 |
| `claude-opus-5` | 5 |
| `gpt-5.3-codex` | 2 |
| `composer-2.5` | 2 |
| `gemini-3-pro` | 1 |
| `grok-code-fast-1` | 1 |
| `gpt-5.1-codex-max` | 1 |
| `glm-5.2` | 1 |

**3. Per-turn `bubble.modelInfo.modelName`.** Present on 904 of 8,000 sampled bubbles,
on both user (`type=1`, 440) and assistant (`type=2`, 464) bubbles.
`modelInfo` has only `modelName`. Patterns in the sample: `claude-4.5-sonnet-thinking`,
`gpt-5.1-codex-high`, `claude-4.5-opus-high-thinking`, `gpt-5-high`, `composer-1`,
`gemini-3-pro-preview`, `gemini-3-pro`, `gpt-5.1`. Public parsers say the user-bubble
value applies to following assistant bubbles until the next user turn.

**4. `usageData` keys** use the `modelName` style (`claude-4.5-opus-high-thinking`,
`gpt-5-pro`, `gemini-3-pro-preview`, …), not the `selectedModels.modelId` style.

**5. `ai_code_hashes.model`** (20,985 of 27,708 rows nonempty): `grok-4.6` (20,831),
`cursor-grok-4.6-xhigh-fast` (128), `claude-opus-5` (26).

**Not observed**

- A `provider`, `providerName` or `providerId` field on composers, bubbles or JSONL
- Anthropic dated snapshot ids (`claude-opus-*-YYYYMMDD`)
- xAI / OpenAI / Google fully qualified ids (`xai/…`, `accounts/…`)
- `claude-fable-*` or `fable-*` (this install has not stored a Fable picker value)
- `auto-smart` (the SDK Router id); Auto is the literal `default`

JSONL has no session-level model.
The only model strings there are `Task` tool inputs (`inherit`,
`cursor-grok-4.6-xhigh-fast`).

**Answer for the spec:** logs store **(a) and (b), not a provider column**. `(a)` is
`modelConfig.modelName` / `modelInfo.modelName` / `usageData` keys: Cursor picker labels
that often include effort (`-high-thinking`, `-xhigh-fast`) and sometimes a `cursor-`
prefix. `(b)` is `selectedModels[].modelId` and some `ai_code_hashes.model` values:
catalog ids such as `claude-opus-4-6`, `composer-2.5`, `grok-4.6`. Neither is the raw
provider API id with a date or regional suffix.
Provider must be inferred from the id family, or looked up in Cursor’s pricing table
(Grok and Composer → Cursor; `claude-*` → Anthropic; `gpt-*` / `o3-*` → OpenAI;
`gemini-*` → Google; `kimi-*` → Moonshot; `glm-*` → Z.ai).

Auto is special: the stored name is `default`, and the served model may be absent.

### Dialects, Accounts and Default Discovery

**Composer chat vs agent.** `unifiedMode` plus `isAgentic` distinguish `chat` from
`agent`. `plan`, `background` and `multitask` are rare extra modes.
JSONL exists only for a subset of `agent` composers.

**Cloud vs local.** Local composers and JSONL use UUIDs.
Cloud shows up as `conversation-search` `cloud-cache` rows (14) and `bc-` keys in
`ItemTable`. There is no local cloud-transcript JSONL tree on this disk.
The SDK persists local agents under a workspace `stateRoot` (SQLite or JSONL) and cloud
agents as `bc-*` ids; that SDK store was not the IDE’s `state.vscdb`.

**Multiple accounts.** One `User/` profile.
`storage.json` has no account-identity values in its key set (only a
`userDataProfilesMigration` flag).
AgentStores includes one numeric `u*` personal store beside per-conversation UUID
stores. Nothing here looks like a second Cursor login’s log root.
A second macOS user, or a second Cursor app-support directory, would be a separate tree.

**Hooks.** `~/.cursor/hooks.json` names events `stop`, `preToolUse`, `postToolUse`,
`afterAgentResponse` and others.
A public feature request says the `stop` payload can include `conversation_id`,
`transcript_path` and a model field, and that it still lacks token usage.
Hooks are a capture path, not a historical store.

**What urollup discovers today.** Default roots are `~/.claude/projects` (and XDG
`claude/projects`) and `~/.codex`. No Cursor path is consulted.
A default `urollup report` on this machine would include **zero** Cursor sessions, even
though 1,022 composers and 265 JSONL files exist.

### Comparison to `claude-project` and `codex-rollout`

| Criterion | `claude-project` | `codex-rollout` | Cursor (this install) |
| --- | --- | --- | --- |
| Default root | `~/.claude/projects/<slug>/<session>.jsonl` | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` | No single root; `state.vscdb` plus optional `~/.cursor/projects/<slug>/agent-transcripts/` |
| Found by current urollup | Yes | Yes | No |
| Writer | One JSONL dialect | One JSONL dialect | JSONL export, SQLite blobs, and search/attribution DBs |
| Session id | `sessionId` on every line and in the file name | Thread id in the file name and `session_meta` | `composerId` UUID; JSONL folder matches when present |
| Request id | `requestId` and `message.id` | `response_id` / `turn_id` on usage records | `requestId` / `usageUuid` on some bubbles; none on JSONL |
| Turn records | One line per content block, with timestamps | Typed `{timestamp, type, payload}` lines | JSONL `{role,message}` plus `turn_ended`; bubbles in SQLite |
| Usage | `message.usage` tokens and cache on assistant lines | `token_usage_record` / `token_count` tokens and cache | Session `usageData.costInCents`; bubble `tokenCount` often zero; no cache |
| Model | `message.model` (provider id) | `turn_context.model` (requested) | Picker `modelName` and catalog `modelId`; Auto is `default` |
| Provider | Implicit Anthropic | Implicit OpenAI | Not stored; infer from id family or Cursor’s table |
| Subagents | Sidecar JSONL + `.meta.json` | Separate rollouts / `inter_agent_communication` | JSONL `subagents/`, `subComposerIds`, header `isSubagent` |
| Ingest shape | Line-oriented, append-only | Line-oriented, append-only | 4.2 GiB live SQLite with WAL; JSONL is incomplete |
| Official schema | Vendor docs + third-party parsers | Codex source types | None; third-party maps only |

What is new for a spec:

- **SQLite is the source of truth**, not the JSONL.
- **Usage is optional and coarse.** Cost sometimes exists; tokens often do not; cache
  never does in the fields inspected.
- **Model is a Cursor catalog/picker string**, sometimes two of them, never a provider
  enum.
- **One conversation can exist in several stores.** Joining on UUID without a precedence
  rule would double-count the 76 composers that also have JSONL.
- **Discovery must be opted in.** A Cursor root is a new default, a new flag, or both.

## Key Insights

Cursor looks like a coding agent with local logs, but it is closer to an editor with a
private database that sometimes writes a thin JSONL sidecar.

The model question the spec asked has a sharper answer than “Cursor marketing name vs
provider id.” The disk stores **Cursor’s own catalog ids**, in two spellings: a
display/effort label (`claude-4.5-opus-high-thinking`) and a shorter id
(`claude-opus-4-5`). Grok and Composer ids are already Cursor-first (`grok-4.6`,
`composer-2.5`, `cursor-grok-4.6-xhigh-fast`). They are not xAI or Anthropic wire ids.
Pricing still needs a provider, and that mapping lives in Cursor’s docs, not in the
rows.

`usageData.amount` is easy to misread as tokens.
The magnitudes on this disk rule that out.
Until Cursor documents the field, treat it as an opaque per-model counter next to
`costInCents`.

`~/.cursor/chats/` is not a safe assumed default.
An adapter that only opened `store.db` would find nothing here, and an adapter that only
opened JSONL would miss substantial composer history.
The exact missing-session count is unresolved in the survey aggregates above.

## Recommendations

1. **Treat a Cursor dialect as a new plan, not an extra root on the existing adapters.**
   The ingest engine is JSONL-oriented; `state.vscdb` is a 4 GiB WAL-backed key-value
   store that must be snapshotted while Cursor is open.
2. **If the goal is token-level reconciliation, do not promise parity.** Local files
   support session lists, model breakdowns from `modelName` / `modelId`, and occasional
   `costInCents`. They do not support Claude-style per-request token and cache
   identities.
3. **If the goal is “what did I run in Cursor?”, prefer `composerData` + `bubbleId`, and
   use JSONL only as a fallback** when a UUID has no composer row (not observed here) or
   as evidence for tool-call text.
4. **Preserve native model evidence at its recorded grain**, and map `default` to Auto.
   Session/model costs take their `usageData` key; bubble tokens take the bubble’s own
   `modelInfo.modelName` when present.
   Current picker and `selectedModels[]` values remain selection metadata, not a
   fallback for historical usage.
   Leave missing historical model attribution unknown; the public parsers’ propagation
   rule needs independent evidence before adoption.
   Do not invent a provider column from marketing copy; keep an explicit lookup table
   keyed on catalog id families, and leave Fable unmapped until a `claude-fable-*` (or
   similar) value is seen.
5. **Do not add `~/.cursor` to default discovery** until the user opts in.
   The tree mixes session logs with extension code, worker logs and private prompts.
6. **Dedup by `composerId`.** JSONL UUID = composer UUID when both exist; JSONL must not
   add a second session.

## Next Steps

- [ ] Decide whether Cursor is in scope for a later phase, given the usage gap
- [ ] If yes, write a plan spec that names the SQLite snapshot rule, the UUID join and
  the model-id mapping table
- [ ] Build sanitized fixtures from synthetic `composerData` / `bubbleId` / JSONL
  shapes; never from this machine’s `state.vscdb`
- [ ] Re-check `~/.cursor/chats/` and `claude-fable-*` on a second install or a later
  Cursor version
- [ ] Cite `uro-890b` from any later Cursor plan spec

## Methodology

Local evidence is from read-only walks and SQLite queries on 2026-09-19 against Cursor
3.21.13 (`CFBundleShortVersionString`). Queries used `sqlite3` URI `mode=ro` while
Cursor was running (an 11 MiB WAL was present).
JSON values were scanned for field names and for string values of keys whose names
contain `model` or `provider`. Those strings were reduced to patterns (lowercase, date
suffixes wildcarded).
Prompt text, file paths, UUIDs and raw counts from usage fields were not copied into
this document.

Public sources were used to name layers this install lacks (`~/.cursor/chats/`) and to
cross-check field names (`modelConfig.modelName`, `modelInfo.modelName`, `usageData`,
`tokenCount`). Where a public reader and this disk disagree, the disk wins for this
version and the disagreement is noted.

Not verified: Windows and Linux path variants beyond the conventional `%APPDATA%/Cursor`
and `~/.config/Cursor` mappings; Cursor versions other than 3.21.13; BYOK rows;
enterprise Router “Displayed” mode writing the served model; Fable picker values; SDK
`stateRoot` layouts for programmatic local agents.

## References

- [Cursor Models & Pricing](https://cursor.com/docs/models) (official; provider column
  and catalog names)
- [Available models](https://cursor.com/help/models-and-usage/available-models)
  (official help; Auto / Router visibility)
- [Cursor TypeScript SDK](https://cursor.com/docs/sdk/typescript) (official; `model.id`,
  `run.usage`, `Agent.getUsage()`, local vs `bc-` cloud ids)
- [Cursor Python SDK](https://cursor.com/docs/sdk/python) (official; same surfaces)
- [Cursor forum: chat history after update](https://forum.cursor.com/t/cant-access-chat-history-since-latest-update/158688)
  (official staff: `~/.cursor/projects/{workspace-slug}/agent-transcripts/`)
- [Cursor forum: full agent transcript](https://forum.cursor.com/t/accessing-the-full-agent-transcript-in-cursor/157311)
  (official staff: JSONL omits tool outputs)
- [Cursor forum: richer transcripts](https://forum.cursor.com/t/richer-agent-transcripts-lifecycle-data-for-observability-langfuse-stop-hooks/166592)
  (JSONL lacks timestamps, usage and per-line model)
- [vibe-replay: Cursor local storage](https://vibe-replay.com/blog/cursor-local-storage/)
  (third-party map; updated 2026-09-14)
- [Agent Sessions: Cursor agent local history](https://jazzyalex.github.io/agent-sessions/guides/cursor-agent-local-history.html)
  (third-party; JSONL + `store.db`)
- [@tracebench/adapter-cursor](https://www.npmjs.com/package/@tracebench/adapter-cursor)
  (third-party; JSONL limitations and `state.vscdb` merge)
- [codeburn Cursor provider notes](https://github.com/getagentseal/codeburn/blob/main/docs/providers/cursor.md)
  (third-party; 180-day bubble lookback, zero token counts)
- [Watchmen PR 118](https://github.com/firstbatchxyz/watchmen/pull/118) (third-party;
  `modelConfig.modelName`, `default` → Auto, tokens remain 0)
- [cc_transcript_viewer `cursor_parser.py`](https://github.com/tim-hua-01/cc_transcript_viewer/blob/main/cursor_parser.py)
  (third-party; IDE DB > `store.db` > JSONL)
- [Portable agent usage brief](research-2026-09-13-portable-agent-usage.md) (urollup
  dialect baseline)
- [urollup design](../../urollup-design.md)
- [urollup discovery](../../../crates/urollup-core/src/adapters/discovery.rs) (default
  Claude and Codex roots only)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
