---
type: is
id: is-01m4fvzray67mxp26jmvbs0qvw
title: "PR #16 B2: first child step after an inherited baseline is not checked against last_token_usage"
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:31.197Z
updated_at: 2026-10-09T08:20:31.197Z
---
Low. Review B https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389. codex_rollout.rs:1134-1137. R4: inherited 90/10, unseeded first counter total=last=300/50 counted as 250 instead of 350, complete, no diagnostic.
