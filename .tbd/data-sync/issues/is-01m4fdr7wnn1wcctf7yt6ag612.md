---
type: is
id: is-01m4fdr7wnn1wcctf7yt6ag612
title: "npm audit fails: braces advisory through tryscript 0.2.1"
kind: bug
status: closed
priority: 1
version: 4
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-10-09T04:11:44.915Z
updated_at: 2026-10-09T09:28:14.069Z
closed_at: 2026-10-09T09:28:14.069Z
close_reason: "Merged to main in 98f9428 via PRs #18 and #22 (stack #23) after senior, correctness, security and follow-up reviews with every finding dispositioned."
resolution: null
duplicate_of: null
---
make npm-audit (and the CI Dependency audit job) now fails on GHSA-vfj7-8cjw-p6xm, a high-severity stack-exhaustion advisory in braces, reached through tryscript 0.2.1 -> fast-glob -> micromatch -> braces. npm reports no fixed braces version; the suggested fix is tryscript 0.3.0, a breaking change. tryscript is first-party, so it is exempt from the 14-day age cool-off. Upgrade it (or drop the fast-glob path), re-run make golden and every golden session, and record the version in SUPPLY-CHAIN-SECURITY.md. Every PR's CI will fail until this lands, including the main branch.

## Notes

Implemented in PR https://github.com/jlevy/urollup/pull/18 (branch fix/compressed-sources-codex-usage, based on main; also applies cleanly on PR 16 with its fork tests passing). Local gates pass except gate proofs and MSRV tests, left to CI. Close when PR 18 merges.
