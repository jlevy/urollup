---
type: is
id: is-01m2yrherhddnx2j22g3xtqedr
title: "Acceptance G5: documented full-history analysis without custom scripts"
kind: task
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies: []
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-20T06:37:29.744Z
updated_at: 2026-09-20T06:37:29.744Z
---
Implement and run the maintained acceptance entry point specified by G5. Use existing watchdog/measurement helpers, one snapshot and structured local run manifest/reports/check results. Prove bounded ingest count, offline artifact queries, joint/marginal consistency, cache populations/TTL, exact fixture list prices, tool deduplication and interval unions, errors and privacy. Synthetic CLI goldens cover the exact documented recipe; private QA requires explicit consent and keeps payloads local. Separate ingest/load/query timings, RSS/physical footprint, host contention and cache-state evidence. Busy live runs are exploratory; fixed-day exclusion is not a snapshot proof. Update guide/status only when demonstrated. Existing uro-ky6c owns 0.1 QA; G5 owns the complete 0.5 workflow.
