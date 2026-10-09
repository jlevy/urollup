---
type: is
id: is-01m4fsy7esdhz11aces5jrtebc
title: "PR #14 A8: Cursor plan quotes the retired 512 MiB threshold"
kind: bug
status: closed
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsxxjgrahm03vd1fr1vhms
hold: null
hold_until: null
created_at: 2026-10-09T07:44:43.992Z
updated_at: 2026-10-09T07:50:32.425Z
started_at: 2026-10-09T07:45:10.895Z
closed_at: 2026-10-09T07:50:32.424Z
close_reason: "Fixed in 712016e: Non-Goals and Rollout Plan refer to ingest acceptance owned by uro-zrr0 and the scalable-ingestion plan instead of the retired 512 MiB threshold. Confirmed by pinned flowmark --auto --check . (exit 0), git diff --check (clean), and the relative link and anchor check of the four changed files (907 links, bad=0)."
resolution: null
duplicate_of: null
---
Severity: Low. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: plan-2026-09-19-cursor-dialect.md :78 ("the 512 MiB peak"), :429 ("the 512 MiB ingest gate").

Problem: the maintainer retired the representative 512 MiB and 10-second thresholds on 2026-09-27 (uro-zrr0); these sentences go stale once that policy reaches main.

Fix: refer to the ingest acceptance owner (uro-zrr0 and the scalable-ingestion plan) without quoting a threshold.
