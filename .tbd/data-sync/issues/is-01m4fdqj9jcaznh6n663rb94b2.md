---
type: is
id: is-01m4fdqj9jcaznh6n663rb94b2
title: Source snapshot losses never reach report coverage or diagnostics
kind: bug
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T04:11:22.800Z
updated_at: 2026-10-09T09:28:14.057Z
closed_at: 2026-10-09T09:28:14.057Z
close_reason: "Merged to main in 98f9428 via PRs #18 and #22 (stack #23) after senior, correctness, security and follow-up reviews with every finding dispositioned."
resolution: null
duplicate_of: null
---
ManifestEntry records coverage failures (corrupt or truncated compressed data, oversized records, read errors, twin fingerprint mismatches) and snapshot-affecting changes (vanished, replaced, truncated, rewritten), but neither adapter turns them into diagnostics or partial coverage: a truncated .jsonl.zst silently reports smaller totals with coverage.complete=true, and the Claude adapter does not even report malformed lines or pending tails. Fix: one shared snapshot_losses pass raises a source-incomplete diagnostic per losing source and an UnreadableSource coverage gap, so totals become partial. Follow-up beyond this fix: Claude malformed-line and pending-tail diagnostics.

## Notes

Implemented in PR https://github.com/jlevy/urollup/pull/18 (branch fix/compressed-sources-codex-usage, based on main; also applies cleanly on PR 16 with its fork tests passing). Local gates pass except gate proofs and MSRV tests, left to CI. Close when PR 18 merges.
