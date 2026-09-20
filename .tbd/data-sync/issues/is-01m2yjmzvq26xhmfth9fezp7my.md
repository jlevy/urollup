---
type: is
id: is-01m2yjmzvq26xhmfth9fezp7my
title: Preserve Phase 3 ordering for the Cursor database adapter
kind: bug
status: closed
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels: []
dependencies: []
parent_id: is-01m2y50zntpg0bavafh8547sfb
created_at: 2026-09-20T04:54:34.102Z
updated_at: 2026-09-20T05:17:05.644Z
closed_at: 2026-09-20T05:17:05.643Z
close_reason: PR14 planning defects fixed, independently reviewed, formatted and pushed as c8befd9be545cccd147a3594721a9fe3fbc327b1. Pinned Flowmark and diff checks passed; no private logs were read. Current main has no hosted workflow, so no CI pass is claimed. Implementation-dependent link/rebase validation remains separately open under uro-knnz.
resolution: null
duplicate_of: null
---
PR14 misreads Decision20 as generic database readers only. Preserve confirmed Phase3 ordering for named Cursor SQLite adapter unless an explicit later exception is recorded.

## Notes

Implemented in cursor-dialect c8befd9. Plan, product pointer and design table now preserve confirmed Phase3 ordering; earlier implementation requires an explicit Decision20 exception. Pinned Flowmark auto/check and git diff --check pass. Awaiting parent integration/review.
