---
type: is
id: is-01m2xcy04ecenmpeaahjma25sa
title: Merge Claude DecodedSources as the discovery-order prefix completes
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T17:55:23.397Z
updated_at: 2026-09-19T17:55:31.501Z
---
After uro-mxcp, loaded whole-history peak is 796 MiB (815,008 KiB). Claude still calls try_read_in_parallel, which retains every DecodedSource (per-source Strings intern table plus Vec<ParsedRecord>) until Corpus::merge absorbs them in discovery order. That join vec sits beside the Codex ledger (611,314 Request rows) for the whole Claude ingest.

Deliver completed sources into a slot array and merge source i as soon as 0..i are present, so earlier intern tables and record vectors drop before later workers finish. Keep discovery-order absorb so worker-count identity holds. Do not re-decode (uro-l3fw already raised peak). try_read_in_parallel_for_each stays for tests; this needs prefix-order merge, not completion-order consume.

Files: sources/parallel.rs (prefix-ready helper); claude_project.rs ingest_discovery_with_workers and Corpus::merge.

Acceptance: fixture snapshots and worker-count identity unchanged; make test; privacy-safe sessions --all on uro-l0gd. Peak must fall.
