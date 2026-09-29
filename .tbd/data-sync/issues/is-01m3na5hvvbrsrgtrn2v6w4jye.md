---
type: is
id: is-01m3na5hvvbrsrgtrn2v6w4jye
title: Accept reusable analytical tables and thin HTML pivot views
kind: feature
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-28-analytical-tables-and-pivots.md
labels: []
dependencies: []
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-29T00:48:51.578Z
updated_at: 2026-09-29T00:51:41.442Z
---
Planning and integration acceptance for analytical data projections, composable filtering/sorting/pivot semantics and a thin plain HTML/CSS local viewer over the existing core query API. Reuse existing summary/bundle/export, joint-query and serve owners; do not create a second accounting engine or portable format. Preserve snapshot identity, exact measures, unknowns, coverage, bounded resource use and private local data. Draft planning only; no implementation authorization and no new alpha release blocker.

## Notes

Draft spec authored with tbd new-plan-spec template and linked from the existing usage-analysis workflow. Defines request/component projections, field/metric catalog, typed filters, sparse pivots, exact/non-additive aggregation, snapshot-bound queries, bounded global sorting/paging, plain HTML/CSS with minimal JavaScript over the existing Rust engine, security, and end-to-end tests. Reuses existing implementation owners. Local engine is the recommended draft approach; standalone HTML arbitrary pivots are not promised. Frontmatter YAML, 20 local links/anchors, pinned Markdown formatting and diff whitespace checks validated. Spec remains local/uncommitted and in review; no runtime feature or dependency added. Existing alpha scope language reconciled to maintainer freeze.
