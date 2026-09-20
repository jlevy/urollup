---
type: is
id: is-01m2yjn0fjtg72ag044th3pzjp
title: Define Cursor historical model attribution precedence
kind: bug
status: closed
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels: []
dependencies: []
parent_id: is-01m2y50zntpg0bavafh8547sfb
created_at: 2026-09-20T04:54:34.737Z
updated_at: 2026-09-20T05:17:05.714Z
closed_at: 2026-09-20T05:17:05.714Z
close_reason: PR14 planning defects fixed, independently reviewed, formatted and pushed as c8befd9be545cccd147a3594721a9fe3fbc327b1. Pinned Flowmark and diff checks passed; no private logs were read. Current main has no hosted workflow, so no CI pass is claimed. Implementation-dependent link/rebase validation remains separately open under uro-knnz.
resolution: null
duplicate_of: null
---
Define historical attribution using bubble or usageData model evidence at its own grain, not the current session selectedModels list; unknown when no historical evidence exists.

## Notes

Implemented in cursor-dialect c8befd9. Historical usage model attribution uses its own usageData key or bubble model field; current selections are metadata, absent historical model/provider remain unknown, and regression fixture cases are specified. Pinned Flowmark auto/check passes. Awaiting parent integration/review.
