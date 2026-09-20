---
type: is
id: is-01m2ygfarr8vb7gzcf96pwzqg3
title: Canonicalize request keys before reread deduplication
kind: bug
status: closed
priority: 2
version: 4
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:16:31.511Z
updated_at: 2026-09-20T05:09:05.593Z
closed_at: 2026-09-20T05:09:05.593Z
close_reason: Fixed on PR4 head2f9afa38acbf4b0f283b9842156950499470e268; all13 hosted checks passed, including Windows/macOS/Linux, MSRV and negative gate proofs. Regression evidence and the two reviewed coverage goldens are recorded in docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md. Propagated copies also passed the integrated local make check; final higher-layer CI remains tracked by stabilization umbrella uro-28fc.
resolution: null
duplicate_of: null
---
R8, PR4: keys promise arbitrary order but reread dedupe precedes key sort. Same evidence and values with keys A,B vs B,A or repeated A produces false ConflictingReread. Canonicalize first and test invariance.
