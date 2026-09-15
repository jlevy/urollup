---
type: is
id: is-01m2k1ywy41c7ss0tkw772acka
title: "PR #3 review R9: totals.unresolved.requests vs unresolved extents ambiguous"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:14.243Z
updated_at: 2026-09-15T17:38:33.726Z
closed_at: 2026-09-15T17:38:33.720Z
close_reason: "R9 fixed in 75228c5: totals.unresolved defined"
resolution: null
duplicate_of: null
---
PR #3 finding R9 (Low). docs/urollup-design.md:1342 shows unresolved requests 0 with two unresolved extents holding 78 requests; :1220 does not say requests counts candidate-set members (4.2, :952-954) and that unresolved extent usage is reported only in the extent. Fix: say so in the totals row and the example comment.
