---
title: Analytical Tables and Thin HTML Pivot Views
description: Reusable reconciled usage data, composable queries, and a bounded local table viewer with shared accounting semantics.
author: Joshua Levy with LLM assistance
date: 2026-09-28
status: Draft; planning only, outside the frozen alpha delivery slice
---
# Feature: Analytical Tables and Thin HTML Pivot Views

**Date:** 2026-09-28 (last updated 2026-09-28)

**Author:** Joshua Levy with LLM assistance

**Status:** Draft. This request authorizes design, not implementation or a new alpha
release gate. Accounting bug fixes and full-history acceptance remain first.

**Tracking:** `uro-ah7a`, under the usage-analysis epic `uro-6kwn`.

## Overview

Read and reconcile agent history once, save the evidence needed for analysis, and query
it through the CLI or a local page.
Users should be able to choose columns, filter, sort, group, pivot, and drill into
sessions without rereading raw logs or writing their own accounting code.

The data model and query semantics belong to the Rust core.
Plain semantic HTML, CSS, and a small JavaScript controller render the results.
A pivot is another arrangement of the same grouped result, not a separate calculation
engine.

This plan refines the
[usage-analysis workflow](plan-2026-09-20-usage-analysis-workflow.md) and the existing
[artifact](../../../urollup-design.md#54-observation-bundles),
[query](../../../urollup-design.md#64-queries-output-formats-and-streams), and
[serving](../../../urollup-design.md#7-serving-layer-optional) designs.
It does not introduce another portable format, database, reconciliation system, or price
engine.

## Goals

- Analyze one named snapshot through interchangeable CLI, agent, and browser clients.
- Compose available dimensions in arbitrary typed filters, multi-column sorts, groups,
  and row/column pivots, subject to explicit resource limits.
- Preserve exact token and cost arithmetic, request identity, unknown values, evidence
  basis, and metric-specific coverage through every operation.
- Make the default session table useful without configuring a pivot, while allowing
  users to save a query and reproduce it later.
- Keep the browser small: bounded result pages, no raw-log ingestion, no authoritative
  aggregation, and no external assets or services.

## Non-Goals

- Implementing any feature during this planning pass or delaying the current alpha for
  the viewer.
- New agent adapters, cloud hosting, synchronization, editing logs, or agent controls.
- A general spreadsheet, formula language, arbitrary SQL console, or plugin framework.
- Inferring absent model/provider/account values to make a pivot appear complete.
- Reconstructing joint observations by joining independent report marginals.
- Loading an unbounded dataset or every possible pivot cell into browser memory.
- Implementing unobserved tool/time metrics or new pricing policies as viewer work.

## Background

Current `report` JSON contains totals and independent dimension breakdowns.
`sessions` contains agent and project labels with session totals, and `daily` contains
dated totals. These are useful views, but they cannot recover a project × model × day
dataset. Separate commands currently ingest live logs independently; their input
boundaries can differ.
The [usage guide](../../../usage-analysis.md) records implemented capabilities.

The existing design already separates portable `UsageSummary` and `BundleManifest`
artifacts from rendered JSON. Summaries lose some query precision; request-level bundles
retain evidence for exact selection and regrouping.
The new analysis surface must use those owners rather than declare today’s report JSON a
reusable database.

Whole-history QA motivates explicit handling of unknown model attribution, ambiguous
ownership, worktree-derived project labels, and suspicious request-size observations.
A polished table is not evidence that those accounting questions are resolved.
Private session contents and usage values do not belong in this spec or its fixtures.

## Design

### Approach and boundaries

```text
Native logs → existing adapters and reconciliation → immutable snapshot / bundle
                                                     │
                                      shared typed query engine
                                                     │
                                      typed result + coverage
                                         ┌───────────┴───────────┐
                                  CLI / agent / export     local HTML table
```

The snapshot owns identities, relationships, counted usage, exclusions, and provenance.
The query engine owns filtering, grouping, aggregation, ordering, pagination, and
pricing selection. Clients own control state and presentation only.

Use the existing optional `serve` layer for interactive whole-history queries.
A user can open a saved bundle with original roots unavailable.
The viewer must not silently add live default roots.
Refresh is explicit and atomically selects a new snapshot; in-flight queries and cursors
continue to identify their original snapshot or fail clearly if it is no longer
retained.

A source manifest records per-file cutoffs, not a claim that all agents were paused at
one wall-clock instant.
Reproducibility comes from the saved reconciled contents and their artifact identity.
Execution timestamps and performance logs remain separate.

### User workflows

| Question | Query and presentation | Correctness requirement |
| --- | --- | --- |
| Which sessions used the most tokens? | Session table, filter agent/project, sort total descending | Sort the full selection before paging; retain unknown-owner usage separately |
| How did model usage change each month? | Month rows, model columns, token measures | Joint request-level grouping, explicit timezone, unknown-model column |
| Which models account for a project’s usage? | Filter project key; group by agent and model | Preserve component model attribution and agent identity |
| Where is caching effective? | Project rows, agent columns, cache-read share | Recompute ratios from compatible known populations, with coverage |
| What did a parent session and its agents use? | Select session with descendants; expand hierarchy | Count overlapping descendants once; distinguish own and inclusive rows |
| Which requests explain a large cell? | Drill from cell to contributing requests and components | Preserve filters, snapshot, unknowns, and stable identity |
| What is the estimated cost by model? | Model rows, cost and unpriced coverage columns | Available only with supported pricing; keep currency and policy separate |

These are examples of one query model, not separate reporting implementations.

### Logical data model

Keep evidence tables in the existing bundle.
Define analytical projections over them, not a second set of independently maintained
accounting facts.

| Projection | Grain and identity | Purpose |
| --- | --- | --- |
| Sessions | One analytical thread ID, scoped by agent/source identity rules | Native session key, parent/relationship evidence, project key and label, known metadata; includes sessions with no counted usage |
| Requests | One reconciled logical request ID | Ownership state, timestamps, full request input size and optional request-scoped metrics; revisions/copies are not extra counted requests |
| Usage components | One selected request/model component, with a stable component key | Disjoint token categories and component-level model/pricing context; ordinary single-model requests have one component |
| Coverage and exclusions | Existing coverage facts and diagnostics at their supported scope | Missing fields, unresolved usage, excluded copies, and reasons why totals are partial |
| Valuations | Existing pricing result for a component and explicit pricing policy | Exact amount, currency, matched/default-assumed/unpriced basis and rate provenance; not another usage fact |

A flat analytical row combines a usage component with its request and known session
dimensions.
This is the useful grain for model pivots, not one row per transcript line or
per session. Component identity derives from reconciled evidence, never an unstable
display row number. A request with unavailable usage remains in the request projection;
do not manufacture a zero-token component to make the tables rectangular.

Ownership may be ambiguous or unknown.
Preserve the known source agent independently of ownership.
Never fan out a request to every candidate owner or assign it to an arbitrary session.
Unowned usage has explicit buckets and does not disappear when a session dimension is
added; explicit session selections follow the core’s existing
counted/possible/unresolved policy.

Session hierarchy is a relationship, not another token measure.
Default session rows show own usage.
Descendant rollups are labeled and use set union when selected roots overlap.
Summing parent-inclusive and child-inclusive rows is not a grand total.

Use stable project keys separately from display labels and recorded working directories.
Matching basenames do not prove project identity.
Worktree consolidation requires an existing evidence-backed or explicit mapping policy;
the viewer does not invent one.

### Field and metric catalog

One versioned catalog describes each exposed field: key, label, type, unit, source
grain, availability, supported operations, aggregation rule, and definition.
Clients use it to populate controls and label results.
Do not build a general schema editor.
Artifact support is reported separately from a supported field whose value is unknown
for a particular row.

Initial dimensions are agent, project, session, parent session, model, effort, timestamp
and derived calendar period.
Provider, billing channel, account, service tier, speed, and geography appear only with
their recorded/mapped/assumed basis and available implementation.
A missing dimension must not silently become a guessed value.

| Metric class | Rule |
| --- | --- |
| Disjoint token categories | Sum ordinary input, cache read, each write lifetime, output, and any supported provider-only category |
| Token subtotals | Total writes is derived from lifetime buckets; reasoning is a subset of output, not another addend |
| Requests and sessions | Exact distinct IDs within the selected population, not the sum of displayed group counts |
| Cache rates | Recompute from retained numerator and denominator populations; never average displayed percentages |
| Cost | Sum exact decimals only within one currency, pricing policy, and valuation basis; keep unknown/default-assumed coverage visible |
| Request sizes and percentiles | Use the request’s full size once per selected request; recompute from retained evidence, never average percentiles or derive size from an advisor component |
| Duration and tool metrics | Expose only when their existing owners implement evidence and aggregation rules; interval union is not ordinary summation |

For optional metrics, return known sum, observed population, unknown population, and
applicability. No observations renders as unknown, not zero.
An observed zero renders as zero.
Mixed coverage renders the known amount with a partial marker.
A metric’s coverage must name whether it counts requests or components.

Model filtering selects matching components for token/cost sums, and the distinct
requests containing those components for request-scoped metrics.
A request split across two models can appear in both model groups, but only once in the
grand request count.
Never copy a request-scoped charge or duration into every component and sum it twice.
Do not silently equate known inclusive input with a fully known cache split.

Preserve the existing artifact portable-integer range and exact decimal-string money
contract. Reject overflow or unsupported precision rather than rounding through
JavaScript numbers. Formatting, units, and abbreviated labels do not change sort keys or
exported values. A broader integer encoding would require a separate versioned contract
decision, not a viewer workaround.

### Query contract

Extend the planned versioned `QuerySpec`; do not add an unrelated browser query format.
CLI flags, saved queries, and UI controls compile to the same normalized representation.
Field names and operators are validated against the catalog before execution.

| Query part | Semantics |
| --- | --- |
| Input | Explicit snapshot/artifact identity and available capabilities |
| Selection | Session scope, agent/project filters, and a half-open absolute time interval |
| Row predicates | Typed equality, membership, comparisons, literal text matching, and explicit missing-value tests; composable AND/OR/NOT |
| Grouping | Ordered tuples for row and column axes; an empty column axis is a grouped table |
| Measures | Supported named aggregations with grain and coverage rules |
| Group predicates | Filters on calculated group measures, evaluated after aggregation; distinct from row predicates |
| Ordering | Multiple typed sort keys, explicit direction and null placement, stable identity tie-breaker |
| Window | Bounded page size/cursor, pivot column window, and result-budget policy |
| Context | Timezone, week start, pricing policy, and schema/query versions |

Unknown-value comparison semantics must be explicit: ordinary comparisons with unknown
yield unknown, and only true predicates select a row.
`is_missing` and `is_present` select those states deliberately; negating an ordinary
comparison does not silently include missing values.
Unavailable fields are rejected, not treated as row-level nulls.

Evaluate selection and row filters, component/request projection, aggregation, group
filters, global ordering, then pagination.
Request-grain filters use request facts; component-grain filters use component facts.
Return totals for the filtered population separately from page subtotals.
Group filters define the retained groups; their overall totals are recomputed from the
union of contributing identities, not summed distinct counts.
Retain a labeled pre-group-filter total when useful for comparison.

Calendar groups derive from retained timestamps using the query’s explicit timezone and
week start, including daylight-saving transitions.
Unknown timestamps retain an unknown-period bucket unless explicitly excluded.
Resolve relative dates once and save absolute bounds.
Exact boundaries that cut a coarse summary bucket require bundle evidence or an explicit
unsupported-precision error; never approximate invisibly.

A pivot is a sparse grouping by `row_axes + column_axes`. Return typed keys, cells,
row/column subtotals, and a separately computed grand total.
Do not materialize a full Cartesian product of dimension values.
A missing cell is not automatically an observed zero.
Expanding a hierarchy or clicking a cell issues a query preserving the snapshot,
filters, and exact contributing group keys.

Saved analysis state comprises the normalized query plus lightweight presentation
choices such as visible columns and column order.
Query identity excludes cosmetic state.
Relative dates do not move when reopening a saved query unless explicitly refreshed.
Existing independent `report` breakdowns remain labeled marginals; they are not silently
reinterpreted as joint rows.

### Result contract and exports

Return a schema version, snapshot/query identity, field definitions, typed rows or
sparse pivot cells, coverage, diagnostics, applied pricing policy, totals, and
pagination/truncation metadata.
Mark exact versus approximate results explicitly; the initial viewer requires exact
queries or a clear refusal.

Query JSON/JSONL is a rendering, not an observation merge input.
CSV exports the full filtered result unless the user explicitly requests the current
page. Include a metadata companion for snapshot, query, units, nulls, and coverage; warn
when exporting partial results.
Preserve exact values, escape delimiters correctly, and neutralize spreadsheet formula
injection in spreadsheet-oriented text exports without mutating the authoritative data.
Reuse existing output-format owners.

### Plain HTML/CSS viewer

Use one table-oriented page with native form controls and a small controller:

- A header names the snapshot, scope, timezone, and partial/unpriced status.
- Filters are visible and removable; an advanced editor composes typed predicates.
- Column controls choose dimensions/measures; clicking a header sets a global sort.
- Group controls choose ordered row and column dimensions for table or pivot mode.
- An ordinary semantic table shows the bounded result, sticky headers, units, and
  sortable unrounded values through the query engine.
- Rows and cells drill down without losing scope; breadcrumbs allow returning to the
  previous query. Coverage details explain unknown or excluded values.
- Save-query, copy-equivalent-CLI, and export actions use the same query/result
  contract.

Start with a session table and optional grouped/pivot mode, not separate dashboard
implementations for every dimension.
Use accessible labels, keyboard navigation, `aria-sort`, visible focus, and textual
status in addition to color.
Keep zero-usage sessions available through the session projection and distinguish them
from sessions whose usage is missing.

HTML/CSS handles layout; JavaScript submits queries, manages control state, and updates
the DOM. No grid framework, charting library, frontend accounting code, CDN, telemetry,
remote font, or browser copy of the full corpus is required.
All native labels and diagnostics render as inert text, never HTML.

The [existing server security controls](../../../urollup-design.md#73-security-controls)
remain mandatory, including loopback binding, authentication, Host/Origin validation, no
CORS, and restrictive response headers.
No arbitrary filesystem-path or SQL endpoint is introduced.
Do not put private filters or credentials into shareable URLs or logs.

Standalone HTML remains the existing later candidate: it may contain a bounded,
precomputed result and presentation-only interactions.
It cannot advertise arbitrary whole-history filtering/pivoting over data it does not
contain. A browser-only query engine or WASM build is not required for the first viewer.

### Resource bounds

Source volume, retained facts, group cardinality, result cells, and DOM rows are
separate budgets. Pagination limits the browser but does not bound a global sort or
group-by.
Account for retained snapshots, distinct-ID sets, grouping state, sort buffers,
concurrent queries, and output serialization under the process-wide policy.

Global sort occurs before paging; sorting the current page alone is incorrect.
The executor must use a bounded exact strategy, with external sorting/partitioning where
needed or a clear resource-limit error before exhaustion.
Spill design and dependency choices require their own evidence; this spec does not
silently pull the deferred ingest spill feature into alpha or introduce a new database.
No sampling, top-N truncation, or discarded groups may masquerade as a complete result.

Bound pivot column cardinality and rendered cells.
On excess, offer narrower filters, fewer axes, column paging, or a streamed long-form
export. Distinguish the full filtered total from the displayed window.
Apply backpressure and cancellation; stale UI responses must not replace a newer query’s
result. Scratch follows the repository’s external-volume policy, and unavailable scratch
fails explicitly.

### Components and API changes

- Core: analytical projections and field/metric catalog; normalized `QuerySpec`; shared
  exact query execution and typed results.
- Artifact owners: extend existing request/bundle contracts only where evidence needed
  for a supported query is missing.
  Preserve deterministic identities and atomic writes.
- CLI: existing planned saved-query, sort/filter, artifact input, and export surfaces.
  Exact flag names and HTTP routes remain interface-review work, not shipped examples.
- Optional serving module: authenticated bounded queries over named snapshots.
- Web assets: plain HTML/CSS and a small controller consuming the public result
  contract.

## Implementation Plan

Two phases, after the current alpha’s accounting and safety work.
Reuse the existing owners; this plan supplies detailed integration acceptance rather
than duplicate tasks.

### Phase 1: Reusable analytical data and queries

- [ ] Review request/component identity, field/metric catalog, and null/coverage rules
  against synthetic mixed-model and ambiguous-ownership cases.
- [ ] Extend the existing contracts, summary/bundle readers and writers, and export
  workflow (`uro-ni7m`, `uro-vgea`, `uro-cye3`).
- [ ] Implement joint grouping and query operators with exact distinct counts,
  deterministic pagination, precision refusals, and resource bounds (`uro-qvp1`).
- [ ] Prove repeated queries and saved queries use one snapshot without raw-log access
  (`uro-x8r8`); reuse CLI/output work (`uro-5qbb`, `uro-rwkw`).
- [ ] Expose pricing and optional metrics only as their existing owners complete them
  (`uro-381f`, `uro-wuby`, `uro-neii`, `uro-f2lv`); unsupported metrics remain explicit.

### Phase 2: Thin viewer and end-to-end acceptance

- [ ] Add the table/pivot controls and optional serving API under `uro-tzjq`, preserving
  the core/server dependency boundary and all existing security requirements.
- [ ] Test global sorts, typed filters, drill-down, pivot totals, query save/reopen,
  cancellation, bounded rendering, and export against the shared core results.
- [ ] Run artifact-only acceptance with original roots inaccessible, then consented
  private-history QA. Keep evidence local and publish no private values.
- [ ] Update the usage guide only with implemented commands and demonstrated limits.

## Testing Strategy

Use a small synthetic oracle: one request contributes 60 tokens to model A and 40 to
model B. A model pivot has two cells and 100 tokens, but a grand total of one distinct
request. Filtering model A returns 60 tokens and one request, not all 100 tokens.
Add another request with missing usage, an observed zero, duplicate/copy records,
unknown ownership, a missing timestamp, and sessions with no counted requests.

Required checks:

1. Joint groups reconcile to marginals for additive measures.
   Distinct counts, percentiles, rates, descendant totals, currencies, and valuation
   policies obey their specific rules rather than summing displayed cells.
2. Filter order, AND/OR/NOT with missing values, numeric versus textual sort, exact
   decimal sort, tie-breakers, and pagination produce the same result as a complete
   small reference calculation.
   Group filters and totals use the declared population.
3. Time boundaries cover daylight-saving changes, non-hour offsets, leap days, and week
   starts. Coarse summaries refuse unsupported exact queries.
4. Saved queries over the same artifact produce identical typed results through CLI and
   HTTP. Snapshot refresh cannot mix pages.
   Original log roots are inaccessible during artifact-only tests.
5. The HTML viewer matches those values, supports keyboard operation, never renders
   untrusted strings as markup, makes no external requests, and cannot bypass auth or
   read arbitrary files.
   CSV injection sentinels remain inert in spreadsheet exports.
6. High-cardinality groups, wide pivots, global sorts, exact distinct sets, overlapping
   queries, cancellation, and exhausted scratch hit explicit bounds without silent data
   loss. Measure ingest, artifact load, query execution, response size, and DOM rendering
   separately. Do not claim query scale from ingestion measurements alone.
7. Existing `make check`, contract checks, CLI goldens, and feature-disabled builds pass
   at the accepted head.
   Viewer tests cannot substitute for unresolved accounting QA.

## Rollout Plan

Keep this design in review while the frozen alpha stabilizes.
Implement the shared data and query layer before the viewer.
Existing consumers keep their report shapes until a documented versioned change is
accepted. The viewer initially exposes only capabilities that the input artifact and
engine actually support.

No new dependency, runtime code, server, schema migration, or private artifact is added
by this planning change.
Full G5 acceptance remains with the broader workflow; a usable table does not establish
complete pricing, tooling, timing, or accounting support.

## Open Questions

- Approve the local-engine-first interactive design, with standalone HTML restricted to
  bounded saved views, rather than requiring serverless arbitrary pivots initially.
- Select measured defaults for query memory, concurrent queries, page size, and pivot
  cell/column limits before implementation acceptance; specify when exact disk-backed
  execution is required rather than refusal.
- Confirm project identity/mapping behavior and how unknown versus conflicting labels
  appear; do not merge worktrees by basename as an incidental UI choice.
- Review exact QuerySpec/result field names and request-component identity against the
  existing contract owners before freezing schemas or adding flags/routes.

## References

- [Usage-analysis workflow](plan-2026-09-20-usage-analysis-workflow.md)
- [Product milestones](plan-2026-09-13-urollup-cli-and-web.md)
- [Scalable ingestion policy](plan-2026-09-16-scalable-ingestion.md)
- [Current usage capabilities](../../../usage-analysis.md)
- [Agent format contracts and accounting incidents](../../../formats/README.md)
- [Full-history QA](../../../../tests/qa/full-history-rollup.qa.md)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
