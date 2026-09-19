---
type: is
id: is-01m2wbbbwc2m0rmc4vzdqtkb67
title: Release discovery tables before the second agent ingest
kind: task
status: in_progress
priority: 0
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
hold: null
hold_until: null
created_at: 2026-09-19T08:08:29.831Z
updated_at: 2026-09-19T08:08:38.988Z
started_at: 2026-09-19T08:08:38.987Z
---
Whole-history peak is about 1.14 GB. Row math (~477 MB of observations plus requests) cannot explain that. The CLI currently ingests Claude, keeps the full Ingested (sources, manifest, threads, relationships, ledger), then ingests Codex, so the peak is Claude-resident plus the Codex working set.

Ingest the larger dialect (Codex) first, copy what SessionIndex needs, then Ingested::release_discovery so sources, manifest entries, threads and relationships are dropped before the other dialect starts. Record UROLLUP_STATS agent counts before release. Keep the session_index stats phase as the sum of both index-and-release steps.

Acceptance: make check; fixture goldens unchanged; UROLLUP_STATS still emits claude_ingest, codex_ingest and session_index.
