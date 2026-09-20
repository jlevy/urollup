---
type: is
id: is-01m2ygfarr8vb7gzcf96pwzqg3
title: Canonicalize request keys before reread deduplication
kind: bug
status: in_progress
priority: 2
version: 3
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:16:31.511Z
updated_at: 2026-09-20T04:24:07.608Z
---
R8, PR4: keys promise arbitrary order but reread dedupe precedes key sort. Same evidence and values with keys A,B vs B,A or repeated A produces false ConflictingReread. Canonicalize first and test invariance.
