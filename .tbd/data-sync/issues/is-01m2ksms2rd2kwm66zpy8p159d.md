---
type: is
id: is-01m2ksms2rd2kwm66zpy8p159d
title: Decide how a request observed only as copies is reported
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - design-decision
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:08.437Z
updated_at: 2026-09-16T00:25:08.437Z
---
Design 3.3 says a copy nested inside another record never counts, and 2.1 says usage that never reaches local logs is an unobserved coverage gap rather than zero. A request whose only observations are copies sits between the two: the usage was really consumed, the original file is gone (Claude Code deletes transcripts after cleanupPeriodDays, and a Codex parent rollout can be archived away), and the copy states what it was.

uro-spce records such a request with Counting::CopyOnly, a copy-without-original diagnostic and no counted usage, and keeps the copies as evidence. Decide whether that stays, or whether a copy-only request should also raise a coverage gap so reports show the missing usage rather than only a diagnostic. See crates/urollup-core/src/ledger/reconcile.rs and the test a_request_seen_only_as_copies_counts_nothing_and_is_diagnosed.
