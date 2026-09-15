---
type: is
id: is-01m2hsnwe1d5z23yy4f53wqk8s
title: "PR #2 review R11: Small contract gaps"
kind: bug
status: closed
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:15.776Z
updated_at: 2026-09-15T16:28:32.344Z
started_at: 2026-09-15T16:13:57.780Z
closed_at: 2026-09-15T16:28:32.343Z
close_reason: "Fixed in 2b6f811: SourceManifest and PriceTable contracts with sources.yaml and prices.yaml in the platform config directory; --since/--until clip summary buckets by bucket start; foreign Origin defined; Windows directory fsync no-op noted; relative times resolved in QuerySpec."
resolution: null
duplicate_of: null
---
Unspecified source manifest and price override config format and location; --since semantics on 15-minute summary buckets; definition of a foreign Origin; directory fsync no-op on Windows; relative time expressions resolved to absolute instants in QuerySpec (squares #11) missing from plan. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
