---
type: is
id: is-01m4h6xnbag7wsd2nstqdan8qx
title: "PR #24 A2: day-row token availability marks agent-specific fields observed on mixed days"
kind: bug
status: in_progress
priority: 2
version: 2
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4h6x4yjxwq1jjk8rvaty17y
hold: null
hold_until: null
created_at: 2026-10-09T20:50:51.364Z
updated_at: 2026-10-09T20:50:59.404Z
started_at: 2026-10-09T20:50:59.402Z
---
Medium, PR #24 review A (https://github.com/jlevy/urollup/pull/24#pullrequestreview-5474469420). tests/parity/local_aggregate.py:83-105, tests/parity/README.md:45-47, tests/qa/milestone-0.1-local-acceptance.qa.md:272-274. A day row carries a field when any request reported it; name labels for day granularity and document the mixed-agent consequence, or carry per-metric request counts.
