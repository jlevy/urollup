---
type: is
id: is-01m2yjmzvq26xhmfth9fezp7my
title: Preserve Phase 3 ordering for the Cursor database adapter
kind: bug
status: in_progress
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels: []
dependencies: []
parent_id: is-01m2y50zntpg0bavafh8547sfb
created_at: 2026-09-20T04:54:34.102Z
updated_at: 2026-09-20T04:57:34.946Z
---
PR14 misreads Decision20 as generic database readers only. Preserve confirmed Phase3 ordering for named Cursor SQLite adapter unless an explicit later exception is recorded.

## Notes

Implemented in cursor-dialect c8befd9. Plan, product pointer and design table now preserve confirmed Phase3 ordering; earlier implementation requires an explicit Decision20 exception. Pinned Flowmark auto/check and git diff --check pass. Awaiting parent integration/review.
