---
type: is
id: is-01m2ygfb2tpbr0xrg39ypezkxj
title: Discard omitted old categories on cumulative counter epoch reset
kind: bug
status: closed
priority: 1
version: 4
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:16:31.832Z
updated_at: 2026-09-20T05:09:05.608Z
closed_at: 2026-09-20T05:09:05.608Z
close_reason: Fixed on PR4 head2f9afa38acbf4b0f283b9842156950499470e268; all13 hosted checks passed, including Windows/macOS/Linux, MSRV and negative gate proofs. Regression evidence and the two reviewed coverage goldens are recorded in docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md. Propagated copies also passed the integrated local make check; final higher-layer CI remains tracked by stabilization umbrella uro-28fc.
resolution: null
duplicate_of: null
---
R9, PR4: RunningTotal merges old baseline after reset. (input100,output100) to (input5,outputNone) to (input10,output1) falsely resets twice and counts input10 rather than5 final delta. Reset must replace baseline; retain merge only within epoch. Reproduce and fix with partial-field tests.
