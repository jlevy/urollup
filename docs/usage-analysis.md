---
title: Usage Analysis and List-Price Estimates
description: Current whole-history commands, cache-token semantics, known limits and the planned reusable analysis workflow.
author: Joshua Levy with LLM assistance
---
# Usage Analysis and List-Price Estimates

urollup currently reports Claude Code and Codex token usage by day, session and selected
report dimensions. The full workflow for jointly slicing calendar periods, providers,
models, cache behavior, tools, time and estimated API cost is planned.
The [workflow plan](project/specs/active/plan-2026-09-20-usage-analysis-workflow.md)
tracks implementation and acceptance.

## Current Commands

From a development checkout, build the current optimized binary and run a report:

```bash
cargo build --locked --release -p urollup
target/release/urollup report --all --group-by model --format json --timezone UTC
```

For local calendar days, choose the IANA timezone you want to use consistently:

```bash
target/release/urollup daily --all --format json --timezone America/Los_Angeles
target/release/urollup sessions --all --format json --timezone America/Los_Angeles
```

Each `sessions` JSON row also has `last_date`, the date in the chosen timezone of its
latest counted request with a timestamp, and `undated_requests`, its counted requests
without one. `daily` buckets requests by the same rule.
A row with no dated request omits `last_date`. A session with a `last_date` before today
and no undated requests had no counted owned usage today.
A request that two sessions both prove they own is ambiguous: it is counted on the
unowned row (`thread: null`) and dates neither session, so a whole-history row can
differ from a `--session` run, which reads only the selected session’s file family.

Default discovery includes the configured Claude roots and Codex’s active and archived
sessions. Source overrides affect which history is included.
Use repeated `--source` options with `--no-default-sources` to restrict input
explicitly. “All” means all sessions discovered from those roots, not every log anywhere
on disk.

Claude Code deletes project transcripts older than `cleanupPeriodDays` (30 days by
default) when it starts, whether or not they are compressed
([transcript storage](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)).
A whole-history Claude report therefore covers about the last month.
To keep more, raise `cleanupPeriodDays` in `~/.claude/settings.json`, or copy
transcripts to an archive directory and pass it with `--source`; urollup reads `.jsonl`,
`.jsonl.zst` and `.jsonl.gz` files there.
If the archive also holds transcripts still under `~/.claude/projects`, add
`--no-default-sources` and name each root explicitly so no transcript is read from two
roots. Codex keeps archived sessions under `archived_sessions`, which default discovery
reads.

Codex totals stay provisional until the real-history comparison is rerun after the
accounting fixes: PR #18 (merged 2026-10-09) and the paginated-fork double count
(`uro-kpbp`) in PR #16. The
[release readiness record](project/specs/active/plan-2026-09-16-first-release-publishing.md#open-correctness-fixes)
lists each open fix.

Each command currently rereads the logs.
Separate commands can observe different input when sessions are active.
`report --group-by project,model` produces separate project and model breakdowns, not
project × model rows.
Only `report` takes `--group-by`; `daily` and `sessions` reject it as a usage error
(exit 2) until joint grouping is implemented.

JSON is useful for inspection and scripts, but a report is a view, not a reusable
request dataset or merge input.
Do not join independent daily and model totals to invent a day × model breakdown.
Report JSON can contain session identifiers, project names, custom model names and
diagnostic detail; keep it local unless you have reviewed it for the intended recipient.

## Capability Matrix

| Need | Current implementation | Planned completion |
| --- | --- | --- |
| Token totals and model breakdown | Available in `report` | Joint dimensions and metric coverage |
| Daily totals | Available in the selected timezone | Group by day together with agent/provider/model |
| Weeks and months | No native command yet; daily token sums can be rebucketed externally | Native calendar queries and explicit week start |
| Claude versus Codex | Session rows identify agent; explicit roots can isolate one agent | First-class agent filter/grouping |
| Provider and account | Provider grouping absent; `report --group-by account` is accepted, but every row is `unknown` because no adapter records an account | Recorded or explicitly mapped provider basis; unknown account retained |
| Cache-read and cache-write tokens | Available; JSON output also splits writes by recorded lifetime | Request hit counts/rates and field-availability coverage |
| Tool calls and usage durations | Not exposed by current reports | Native evidence, tool counts and separate time measures |
| API list-price estimate | Not implemented | Reviewed rates, request-level matching and coverage |
| Requery without original logs | Not implemented | Existing summary/bundle and export design |

## Cache Tokens and Cache Hits

JSON output uses these token fields and omits a field that no counted request reported:

| Field | Meaning |
| --- | --- |
| `uncached_input` | Input outside the cache-read/write categories |
| `cache_read` | Input tokens reported as served from cache |
| `cache_write_5m` | Cache writes with recorded five-minute lifetime |
| `cache_write_1h` | Cache writes with recorded one-hour lifetime |
| `cache_write_unspecified` | Cache writes whose lifetime is not established |
| `cache_write` | Sum of the three write categories; do not add it again to those categories |
| `output` | Generated output, including reasoning when reported as a subset |
| `reasoning` | The reasoning subset; do not add it again to output |
| `provider_only` | Provider-specific additive tokens; the Claude and Codex adapters report none |
| `total` | Sum of the disjoint categories: uncached input, cache reads, the three write lifetimes, output and `provider_only`; `cache_write` and `reasoning` are already inside it |

Tables show at most one cache-write total, and the `sessions` table folds every input
category into one column; the write lifetimes appear only in JSON.

Claude input counters exclude cache reads and writes.
Codex input counters include cached input.
When a record reports its cached split, the adapter subtracts it to derive uncached
input; a record without that split keeps its whole input as uncached input, which the
[workflow plan](project/specs/active/plan-2026-09-20-usage-analysis-workflow.md#cache-accounting-and-list-price-estimates)
plans to keep as an unknown allocation instead.
The ledger normalizes these meanings before summing.
Reconciliation handles repeated usage observations rather than summing every raw log
line.

The five-minute and one-hour categories are cache-write lifetimes, not observed internal
cache levels. Unreported lifetimes remain unspecified.
If a flat write count conflicts with a lifetime breakdown, the adapter keeps the flat
amount as unspecified and records a diagnostic; it does not invent a matching split.

A cache read is token usage within a model request.
It is not a separate read API call.
A request can read existing cache and write new cache in the same response.
Generated output is another token category, not a cache write.

Two useful rates answer different questions:

- **Token cache-read share:** cache-read tokens divided by inclusive input tokens for a
  population with known input components.
- **Request cache-hit share:** requests with positive cache reads divided by requests
  with observed cache-read status.
  Requests whose status is unknown need a separate count.

The current CLI exposes token sums, not the second rate’s request populations.
Those counts cannot be recovered from token totals alone.
An absent field is not zero; current aggregate sums also do not say how many requests
lacked each field. `coverage.complete` describes the implemented request-accounting
policy, not complete observation of every cache, tool, timing or pricing dimension.

## List-Price Valuation

A list-price estimate answers what the recorded usage would cost under a stated API rate
table.
It stays separate from personal-plan payments, configured subscription allocations
and any recorded provider charge.

For each priced request, multiply each disjoint category by its applicable rate per
million tokens, sum those amounts, and divide by one million:

```text
uncached input × input rate
+ cache reads × cache-read rate
+ five-minute writes × five-minute-write rate
+ one-hour writes × one-hour-write rate
+ output × output rate
```

Unspecified write lifetimes and any provider-specific category require their own
supported rate or a labeled assumption.
Reasoning is included in output once.
A cache-hit percentage is descriptive; it is not another priced item.

Correct matching also needs exact model attribution, provider/billing channel, request
date, recorded service tier or speed and any applicable per-request context band.
Current adapters do not yet retain every one of those dimensions.
All-time model totals are insufficient when rates or tiers change or individual requests
cross a pricing threshold.
Multi-model requests also need their usage components priced separately; a model
breakdown’s request counts are not necessarily additive across models.

The planned workflow distinguishes historical rates at each request date from a chosen
price-table date applied to the entire history.
It records the table version, currency, aliases and overrides, and reports priced,
default-assumed and unpriced coverage.
Unknown usage has unknown cost; it is never silently valued at zero.
See [the pricing design](urollup-design.md#45-price-table) and `uro-wuby`, `uro-neii`
and `uro-381f`. No current dollar rates or cost totals are claimed by this guide.

## Planned Reusable Workflow

G5 requires a documented first-party path to:

1. Discover and reconcile selected sources once at a recorded snapshot boundary.
2. Save the planned summary/bundle formats with the dimensions and metric coverage
   needed for supported queries.
3. Query day/week/month × agent/provider/model against that same artifact without
   reading live default roots, including cache, tool, time and pricing measures as their
   implementations become available.
4. Write organized reports and a validation record with explicit failures and gaps.

A summary with coarse time buckets cannot answer every exact local-time boundary.
Queries use request-level bundle evidence when needed or explicitly state the precision
limit. Session span, API duration sums, busy interval unions and tool call-to-result
intervals remain distinct; they must not be collapsed into one “wallclock” total.

The [full-history QA playbook](../tests/qa/full-history-rollup.qa.md) owns current
manual checks. Its maintained-runner follow-up must avoid per-session rescans, as the
`make e2e-local` aggregate now does, keep private payloads local and separate
exploratory live runs from reproducible acceptance.
Runtime, RSS, physical footprint and usage durations are different metrics.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
