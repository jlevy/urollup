---
type: is
id: is-01m2ygfbcmzqfh932496b7ydch
title: Keep oversized unterminated input pending within the snapshot cutoff
kind: bug
status: closed
priority: 2
version: 4
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
hold: null
hold_until: null
created_at: 2026-09-20T04:16:32.147Z
updated_at: 2026-09-20T05:09:05.617Z
started_at: 2026-09-20T04:19:29.927Z
closed_at: 2026-09-20T05:09:05.617Z
close_reason: Fixed on PR4 head2f9afa38acbf4b0f283b9842156950499470e268; all13 hosted checks passed, including Windows/macOS/Linux, MSRV and negative gate proofs. Regression evidence and the two reviewed coverage goldens are recorded in docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md. Propagated copies also passed the integrated local make check; final higher-layer CI remains tracked by stabilization umbrella uro-28fc.
resolution: null
duplicate_of: null
---
R10, PR4: reader returns Oversized at EOF without newline; advance adds nonexistent newline. max4 input {} newline12345 yields cutoff9 for8bytes and loses pending tail. Preserve pending length5 cutoff3, plus oversized diagnostic; test plain/compressed boundaries.
