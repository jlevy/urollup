---
type: is
id: is-01m2ygfbcmzqfh932496b7ydch
title: Keep oversized unterminated input pending within the snapshot cutoff
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies: []
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:16:32.147Z
updated_at: 2026-09-20T04:16:32.147Z
---
R10, PR4: reader returns Oversized at EOF without newline; advance adds nonexistent newline. max4 input {} newline12345 yields cutoff9 for8bytes and loses pending tail. Preserve pending length5 cutoff3, plus oversized diagnostic; test plain/compressed boundaries.
