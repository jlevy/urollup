---
type: is
id: is-01m4h6xk06txm1spbd47n460nw
title: "PR #24 A1: new query tests need a tz database and fail on Windows"
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
created_at: 2026-10-09T20:50:48.955Z
updated_at: 2026-10-09T20:50:58.874Z
started_at: 2026-10-09T20:50:58.873Z
---
Blocker, PR #24 review A (https://github.com/jlevy/urollup/pull/24#pullrequestreview-5474469420). crates/urollup-core/src/query/aggregate.rs:801,838 resolve America/Los_Angeles through TimeZone::get; Windows CI has no tz database (job 113982970162). Build the zone from a POSIX TZ string in the tests.
