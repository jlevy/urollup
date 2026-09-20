---
type: is
id: is-01m2ygfaenzm3zrfran0wbkws6
title: Keep local QA operating-system failures free of private paths
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
created_at: 2026-09-20T04:16:31.188Z
updated_at: 2026-09-20T05:12:15.786Z
closed_at: 2026-09-20T05:12:15.785Z
close_reason: R7 fixed on PR8 head 4a8ef34cd284661b2b92ed7137837232c7946bef. All 13 final hosted checks passed, including platform tests and gate proofs; synthetic path-leak regressions passed and no private logs were accessed.
resolution: null
duplicate_of: null
---
R7, PR8: aggregate/parity OS errors expose traceback and private paths. Catch at CLI boundary with fixed diagnostics and test synthetic nonexecutable paths and publication failures.
