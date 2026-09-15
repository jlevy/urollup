---
type: is
id: is-01m2k1yvvc80a1h5d0qmss31j2
title: "PR #3 review R6: --no-index forfeits cross-thread overlap detection, unstated"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:13.131Z
updated_at: 2026-09-15T17:38:31.027Z
closed_at: 2026-09-15T17:38:31.022Z
close_reason: "R6 fixed in 75228c5: --no-index consequence stated in 5.2, 5.3 and Decision 17"
resolution: null
duplicate_of: null
---
PR #3 finding R6 (Medium). docs/urollup-design.md:1364-1366 marks index-less extents with different owner threads disjoint; :1391-1392 lists one request under different threads as needing observations, but without an index nothing detects it and the request counts twice. Decision 17 tradeoffs :2435-2436 read as a safety guarantee. Fix: state the consequence in the 5.2 --no-index bullet (:1233-1235), 5.3 and Decision 17 tradeoffs.
