---
type: is
id: is-01m2ygfb2tpbr0xrg39ypezkxj
title: Discard omitted old categories on cumulative counter epoch reset
kind: bug
status: in_progress
priority: 1
version: 3
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:16:31.832Z
updated_at: 2026-09-20T04:24:07.616Z
---
R9, PR4: RunningTotal merges old baseline after reset. (input100,output100) to (input5,outputNone) to (input10,output1) falsely resets twice and counts input10 rather than5 final delta. Reset must replace baseline; retain merge only within epoch. Reproduce and fix with partial-field tests.
