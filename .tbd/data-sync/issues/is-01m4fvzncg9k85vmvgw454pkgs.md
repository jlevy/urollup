---
type: is
id: is-01m4fvzncg9k85vmvgw454pkgs
title: "PR #16 A1: copy-only completeness fires when the parent is present (unkeyed copied token_count in direct-usage files)"
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:28.175Z
updated_at: 2026-10-09T08:20:28.175Z
---
High. Review A https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467191112. totals.rs:138-140, :207-209; codex_rollout.rs:1077-1103, :1006-1013. Probes P4 (0.153+ legacy-destination fork, parent present) and P5 (paginated child, boundary 2, copied token_count at ordinal 1) report 120 tokens but complete=false with no diagnostics.
