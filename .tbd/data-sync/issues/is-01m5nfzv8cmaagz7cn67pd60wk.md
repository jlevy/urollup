---
type: is
id: is-01m5nfzv8cmaagz7cn67pd60wk
title: Show own and descendant usage separately in report
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - cli
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:59.691Z
updated_at: 2026-10-23T23:01:59.691Z
---
Design §4.2 and §6.1 say reports show own and descendant usage separately, but report folds the selected union (descendants by default) into one total; sessions gives per-thread rows. Add a self/descendants split to report totals in table and JSON, as the §6.6 session report example shows.
