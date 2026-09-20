---
type: is
id: is-01m2ygfa3ggcph5evsdcgx0w98
title: Preserve Windows paths and arguments when launching the golden runner
kind: bug
status: in_progress
priority: 2
version: 2
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies: []
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T04:16:30.831Z
updated_at: 2026-09-20T04:17:42.637Z
---
R6, PR4: scripts/run-golden.mjs invokes Windows .cmd with shell:true and unquoted paths. Launch locked tryscript JS via process.execPath and shell:false; test spaced and metacharacter paths.
