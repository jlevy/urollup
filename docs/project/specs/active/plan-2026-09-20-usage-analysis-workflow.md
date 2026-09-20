---
title: Whole-History Usage Analysis Workflow
description: Make calendar, provider, model, cache, tool, time and list-price analysis a reproducible workflow over one reconciled snapshot.
author: Joshua Levy with LLM assistance
date: 2026-09-20
status: Planned; current CLI supports token reports, with workflow and pricing gaps
---
# Feature: Whole-History Usage Analysis Workflow

## Overview

A user should be able to read their Claude Code and Codex history once, then inspect
usage by day, week or month, agent, provider and model without writing Python glue or
repeatedly decoding the original logs.
The same result should support cache analysis, tool counts, measured durations and a
clearly labeled API list-price estimate.

Epic `uro-6kwn` owns this workflow and its acceptance goal G5 (`uro-i6xb`). It composes
the [product milestones](plan-2026-09-13-urollup-cli-and-web.md) and the existing
summary, bundle and pricing work; it does not introduce another storage format or
accounting engine. The [usage guide](../../../usage-analysis.md) owns user-facing
instructions.

The narrower 0.1 alpha still delivers uncached token reports.
Whole-history analysis across every facet is accepted only when G5 passes; a green 0.1
gate does not imply it.
Persistent caching and the web UI retain their existing phase boundaries.

## Goals

- A documented, first-party workflow discovers the selected roots, reconciles once and
  produces consistent views from one snapshot.
- Reusable summaries or bundles let subsequent queries run with the original log roots
  unavailable. No per-session subprocess loop or user-authored transformation is needed.
- Calendar and categorical dimensions compose: a week × agent × provider × model query
  produces joint rows, not independent marginal totals.
- Cache metrics preserve token classes, write lifetimes, unknowns and useful request
  counts. List-price estimates preserve the assumptions needed to interpret them.
- Tool and time measures have evidence, coverage and aggregation rules that prevent
  replayed calls and concurrent intervals from being counted twice.
- One maintained acceptance entry point produces organized local results, comparisons
  and performance evidence without mixing incompatible snapshots.

## Non-Goals

- Treating estimated API list price as a subscription bill or actual provider charge.
- Inferring a cache hit from elapsed time, a tool execution duration from a wait, or a
  provider from an agent name without an explicit attribution basis.
- Adding a second ad hoc JSON database alongside the planned summary and bundle formats.
- Making missing data look complete by filling it with zero or silently dropping a
  requested dimension.
- Moving Cursor or other later adapters into the 0.1 scope.

## Background and Review Findings

The review is of the reporting surface on merged main `4c55617` and the local workflow,
not a new full accounting-core audit.
That revision passed all 15 hosted CI jobs.
Private usage counts, model labels, timings and local result paths remain local.

| Finding | Severity | Evidence and consequence | Planned fix |
| --- | --- | --- | --- |
| R1: Grouping options can do nothing | Medium | `SelectionArgs.group_by` is shared by all three commands, but `execute` reads it only for `report`. On a synthetic fixture, `daily --group-by model` exits successfully with the same daily-only JSON. | Reject unsupported combinations now; implement joint grouping through the common query engine. |
| R2: Each view starts another ingest | High for G5 | `execute` constructs a new `Corpus` for every invocation; separate daily and model reports cannot recover the missing day × model relationship. | Query one in-memory reconciled snapshot and reuse the existing portable artifacts for later invocations. |
| R3: Local aggregate tooling does not scale with session count | High | `stable_session_summary` invokes `daily --session` for every session, after three full commands. Its metric allowlist also omits cache-write lifetimes and coerces missing counters to zero. | Replace the per-session loop, preserve metric availability, and test invocation count against corpus size. |
| R4: Pricing context is not retained end to end | High for pricing | Token counters distinguish cache lifetimes, but `Request` and the adapters do not retain service tier or speed. Report marginals omit per-request pricing context. | Add evidence-backed pricing fields before pricing or serializing reusable analysis artifacts. |
| R5: The benchmark procedure needs a maintained owner | Medium | A local driver orchestrated multiple live-log commands and a separate script derived calendar views. Busy-host timings and growing input do not prove immutable-snapshot determinism or performance acceptance. | A checked-in harness with a run manifest, a stable input boundary and explicit incomplete/failed checks. |

The ledger’s disjoint token categories, reasoning-as-output-subset rule, copy
reconciliation and shared query functions are useful foundations.
No replacement of those primitives is proposed.
The missing layer is a complete query and artifact workflow around them.

## Design

### One snapshot and one query engine

Resolve source discovery through the production discovery code, including configured
overrides, archived and compressed sources.
Record the source selection, immutable snapshot boundary, engine revision, normalized
query, timezone and week start locally.
Every view in an analysis run uses that same reconciled ledger.
Wall-clock benchmark metadata belongs to a separate run record, outside deterministic
artifact identity.

Extend the library entry point so several normalized queries can share a corpus without
re-decoding it. The CLI workflow uses the existing `export` and `--source` surfaces when
they land. Querying a saved artifact must not implicitly mix in current default logs;
document explicit `--no-default-sources` until the interface enforces that choice.
A convenience command or multi-query flag, if needed, must have a documented contract
and CLI goldens before it is advertised.

Use `UsageSummary` for supported additive views and `BundleManifest` request tables for
queries requiring finer evidence.
A summary reader must declare dimensions and precision it lacks.
Fifteen-minute UTC buckets alone cannot promise exact arbitrary local-date boundaries or
arbitrary timestamp cutoffs; use retained request timestamps when a requested boundary
cuts a bucket, or return an explicit unsupported/approximate result.
Never silently rebucket a partial bucket as exact.

### Joint grouping and metric coverage

Represent grouping as a tuple of dimensions in the normalized query and result rows.
Calendar grouping supports explicit timezone and week start, including daylight-saving
transitions. Unknown agent, provider, model, timestamp and ownership values retain
explicit groups. Multiple requested dimensions must not be interpreted as separate
one-dimensional breakdowns without labeling that form.

Agent is the application; provider is the attributed model service.
Preserve observed, explicitly mapped and unknown provider bases, with a version on any
model mapping. Billing channel remains distinct from provider.
Account stays unknown when absent.
Share this general facet contract with the later Cursor work; it must not depend on
Cursor database ingestion.

Preserve multi-model usage components within a logical request, including advisor usage.
Attribute and price each component at its own model’s rate.
Distinct request counts across model or tool groups can overlap; label them non-additive
and retain the request index needed for exact regrouping.
Do not force a sum of those counts to equal the global distinct-request count.
Cache-hit request populations must use the same declared scope as their numerator and
denominator.

For every optional measure, keep both known sums and availability counts.
An all-zero sum, an unreported category, a partially observed sum and an unsupported
measure are different results.
Overall request coverage does not establish coverage of every metric.
Persist the numerator and denominator behind each rate so regrouping recomputes the rate
rather than averaging percentages.

### Cache accounting and list-price estimates

Keep uncached input, cache-read input, 5-minute writes, 1-hour writes,
unspecified-lifetime writes and output as disjoint categories.
Reasoning remains a subset of output.
Total cache writes equal the three write categories, not another additive token class.
Cache reads already measure tokens served from cache; do not add “cache hits” as another
token quantity.

Add request-level cache metrics: counted requests with positive reads, observed zero
reads and unknown reads, and the corresponding write metrics.
A request may both read and write cache.
These are classifications of model requests, not separate cache API calls.
Define request hit share over requests with observed cache-read status, and token read
share over inclusive input for the population whose input categories are known; report
exclusions and unknown denominators explicitly.
When native input includes cache reads but the cache split is absent, preserve the known
inclusive amount and unknown allocation.
Pricing must not silently treat all of that input as proven uncached usage; add a
missing-split normalization regression.

The existing pricing work (`uro-wuby`, `uro-neii`) owns decimal arithmetic and rates.
Its input must retain provider and billing-channel basis, exact model and
served/requested basis, timestamp, service tier/speed, inclusive request size, cache
lifetimes and token components.
Price per request or an equivalent lossless pricing partition before aggregating.
Whole-history model totals cannot recover long-context thresholds, tier changes or
effective dates.

Offer distinct, labeled valuation policies: historical list price at the request date,
and a chosen price-table date applied as a counterfactual across history.
Neither is an actual bill.
Record the table version, date, currency and configured overrides.
Keep priced, default-assumed and unpriced amounts and their coverage separate.
Missing usage has unknown monetary value, not a zero-dollar cost.
Unknown lifetime or tier requires an explicit documented assumption; an observed
unrecognized value stays unpriced.

Pricing regression cases cover mixed models and advisor components, standard and
recorded faster tiers, both cache-write lifetimes, unknown lifetime, partial usage,
context thresholds, rate changes and reasoning inclusion.
Use invented fixture rates with exact expected decimals.

### Tools and usage time

Native adapters must produce tool actions and timing evidence, not merely declare ledger
types. Dedupe tool actions with evidence-backed identities across replay, resume and
forks. Preserve native names and normalized categories and distinguish missing call
results from recorded outcomes.

Report session span, sum of observed API/request durations, union of known busy
intervals and tool call-to-result intervals separately.
A tool interval includes scheduling and permission waits; it is not execution time.
First/last observed request records do not prove an API duration.
Unknown endpoints stay unknown.
Concurrent spans and busy time are non-additive; recompute interval unions when grouping
or merging.
Duration intervals clip to calendar boundaries, while calls and tokens follow
their recorded attribution policy.
The benchmark’s command runtime is a separate measure.

### Organized results and validation

A maintained runner writes a fresh local output directory with a manifest, named
reports, phase timings, machine-readable check results and diagnostics.
Artifact publication is atomic.
Interrupted or partial runs cannot masquerade as complete ones.
Record fixture or private-input provenance locally without placing prompts, IDs, paths,
custom model labels or usage aggregates in shared issues, docs or CI artifacts without
explicit approval for that destination and payload.

Use the existing watchdog and measurement helpers.
Do not introduce another measurement engine or import helpers through scratch-script
module loading. Distinguish process RSS from macOS physical footprint; record both where
available. Record build configuration, worker count, host contention, input boundary and
cache state. “First run” is not proof of a cold filesystem cache.
Build time and postprocessing time stay separate.

## Implementation Plan

One workstream, delivered through the existing milestones.
Existing artifact, pricing and report owners retain their scopes.
`uro-gtu4` owns this planning/documentation pass, not completion of the unchecked
implementation work below.

- [ ] Correct misleading CLI options and report help in 0.1 (`uro-oz6w`).
- [ ] Replace the local aggregate helper’s per-session scans and preserve cache detail
  and unknowns in its versioned output; retain consent and privacy tests (`uro-qg1a`).
- [ ] Add joint calendar/agent/provider/model grouping and unknown groups (`uro-qvp1`).
- [ ] Retain pricing context and add cache-request metrics with per-metric coverage
  (`uro-381f`).
- [ ] Share a reconciled snapshot across views; extend the existing summary/bundle and
  export work (`uro-ni7m`, `uro-vgea`, `uro-cye3`) for reusable analysis (`uro-x8r8`).
- [ ] Produce native tool/timing evidence (`uro-f2lv`) and expose it through `uro-rwkw`;
  reuse `uro-i6o2` for taxonomy and captured-stream integration.
- [ ] Complete existing reviewed rates and matching (`uro-wuby`, `uro-neii`), adding
  explicit historical-versus-chosen-date valuation and coverage tests.
- [ ] Complete G5 through the maintained runner and update the user guide from planned
  behavior to tested commands only as each surface ships (`uro-i6xb`).

Dependencies: `uro-x8r8` waits on export and joint grouping; `uro-neii` waits on pricing
evidence; `uro-rwkw` waits on native tool/time evidence.
G5 waits on those completed surfaces and the local helper/CLI corrections.
The later Cursor facets (`uro-2qxq`) reuse the generic grouping work; the dependency
does not move Cursor’s phase boundary.
G1 also waits on the local-helper correction, and G3 includes the CLI honesty fix.

## Testing Strategy

**G5: whole-history analysis without ad hoc scripts.** On a hermetic mixed-agent corpus,
then on explicitly consented local input:

1. One documented ingest/reconcile workflow produces all supported views against one
   snapshot. Instrumentation proves ingestion count is independent of view/session count.
2. Requery a saved artifact with original roots inaccessible; no read of live defaults
   occurs. Joint calendar × agent × provider × model totals reconcile with marginals,
   keeping unknown groups and metric coverage visible.
3. Verify calendar boundaries, cache lifetimes and hit denominators, exact fixture
   prices, unknown fields, ambiguous ownership, replayed tool actions and overlapping
   intervals. Price and time totals obey their own aggregation rules.
4. Compare one/eight-worker results against the same immutable input; cross-view
   arithmetic and external parity are separate checks.
   Live-day exclusion alone does not establish snapshot equality.
   Explain every parity residual.
5. Run the existing full-history performance gates on a suitable host.
   Record ingest, artifact load and query/render time separately; do not accept missing
   features or reduced input to make the gate pass.
   Establish a repeat-query target from the first implemented artifact benchmark; the
   existing ingestion targets remain in force.
6. Exercise the complete documented workflow in CLI goldens.
   No custom Python/`jq` stitching, per-session invocation loop or hidden schema
   conversion is required.
7. Prove failure behavior for unsupported dimensions, partial runs, changed source
   boundaries, unpriced usage and unavailable timing data.
   Preserve private results locally and test any shareable projection with privacy
   sentinels.

## Rollout Plan

Fix current CLI honesty and local QA defects in the 0.1 workstream.
Reusable artifacts remain milestone 0.2; prices remain 0.4; the complete CLI analysis
surface and G5 belong to 0.5. Implementation sequencing can be refined without declaring
G5 complete early. No pending maintainer policy is silently accepted by this plan.
Pricing-policy details in design §4.5 and unknown-cache-lifetime policy retain their
recorded review status.

## References

- [Design: measure contracts](../../../urollup-design.md#41-measure-contracts)
- [Design: prices](../../../urollup-design.md#45-price-table)
- [Full-history QA](../../../../tests/qa/full-history-rollup.qa.md)
- [Scalable ingestion](plan-2026-09-16-scalable-ingestion.md)
- [First release](plan-2026-09-16-first-release-publishing.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
