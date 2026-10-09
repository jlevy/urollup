---
type: is
id: is-01m4fdqj9jcaznh6n663rb94b2
title: Source snapshot losses never reach report coverage or diagnostics
kind: bug
status: in_progress
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T04:11:22.800Z
updated_at: 2026-10-09T04:11:47.537Z
---
ManifestEntry records coverage failures (corrupt or truncated compressed data, oversized records, read errors, twin fingerprint mismatches) and snapshot-affecting changes (vanished, replaced, truncated, rewritten), but neither adapter turns them into diagnostics or partial coverage: a truncated .jsonl.zst silently reports smaller totals with coverage.complete=true, and the Claude adapter does not even report malformed lines or pending tails. Fix: one shared snapshot_losses pass raises a source-incomplete diagnostic per losing source and an UnreadableSource coverage gap, so totals become partial. Follow-up beyond this fix: Claude malformed-line and pending-tail diagnostics.
