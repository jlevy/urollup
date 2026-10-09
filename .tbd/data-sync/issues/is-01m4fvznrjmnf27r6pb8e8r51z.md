---
type: is
id: is-01m4fvznrjmnf27r6pb8e8r51z
title: "PR #16 A2: a per-rollout fork-boundary anomaly aborts every report, Claude included"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:28.561Z
updated_at: 2026-10-09T08:20:28.561Z
---
Medium. Review A https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467191112. codex_rollout.rs:950-956, :993-998, :1127-1133; adapters.rs:62-71; cli.rs:412-418. P2 (paginated-subagent fixture without token_usage_record) and P3 (limits-only token_count without ordinal) abort the whole run. Coordinator decision: degrade per rollout (copied history + diagnostic + coverage gap); limits-only token_count must not trip the ordinal check.
