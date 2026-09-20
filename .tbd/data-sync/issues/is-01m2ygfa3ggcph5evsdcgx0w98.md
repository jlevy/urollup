---
type: is
id: is-01m2ygfa3ggcph5evsdcgx0w98
title: Preserve Windows paths and arguments when launching the golden runner
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
created_at: 2026-09-20T04:16:30.831Z
updated_at: 2026-09-20T05:09:05.581Z
closed_at: 2026-09-20T05:09:05.580Z
close_reason: Fixed on PR4 head2f9afa38acbf4b0f283b9842156950499470e268; all13 hosted checks passed, including Windows/macOS/Linux, MSRV and negative gate proofs. Regression evidence and the two reviewed coverage goldens are recorded in docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md. Propagated copies also passed the integrated local make check; final higher-layer CI remains tracked by stabilization umbrella uro-28fc.
resolution: null
duplicate_of: null
---
R6, PR4: scripts/run-golden.mjs invokes Windows .cmd with shell:true and unquoted paths. Launch locked tryscript JS via process.execPath and shell:false; test spaced and metacharacter paths.
