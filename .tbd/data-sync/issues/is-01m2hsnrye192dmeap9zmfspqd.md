---
type: is
id: is-01m2hsnrye192dmeap9zmfspqd
title: "PR #2 review R6: Summary cover rule not computable as specified"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:12.204Z
updated_at: 2026-09-15T16:28:31.137Z
started_at: 2026-09-15T16:13:57.687Z
closed_at: 2026-09-15T16:28:31.135Z
close_reason: "Fixed in 2b6f811: nonfinal is a request ID to revision map included in the digest; cover compares revisions only for nonfinal requests; null-thread extents use candidate-set equality and are never disjoint without an index."
resolution: null
duplicate_of: null
---
Cover needs usage revisions but indexes hold IDs only, nonfinal is undefined as a structure, and same-thread is undefined for null-thread ambiguous extents. Define nonfinal as request ID to revision, compare revisions only for nonfinal requests, and define cover and disjoint for null-thread extents by candidate-set equality. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
