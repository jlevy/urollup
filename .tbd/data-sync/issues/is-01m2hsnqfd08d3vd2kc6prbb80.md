---
type: is
id: is-01m2hsnqfd08d3vd2kc6prbb80
title: "PR #2 review R4: Phase 1 capture-on-every-run write cost unmeasured"
kind: bug
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:10.696Z
updated_at: 2026-09-15T16:28:30.271Z
started_at: 2026-09-15T16:13:57.634Z
closed_at: 2026-09-15T16:28:30.269Z
close_reason: "Fixed in 2b6f811 (maintainer decision): Phase 1 captures only sources idle for --capture-idle (5 minutes default); unwritable store continues with a diagnostic; latency gates exclude capture writes and capture-write throughput is recorded separately."
resolution: null
duplicate_of: null
---
Phase 1 rewrites a complete capture entry for every changed source on every run via the slow Value parse path; only a combined zstd-3/19 build was timed. Measure capture-write throughput, avoid O(source) recapture for active sessions, state whether gates include capture writes, and degrade to --no-capture with a diagnostic when the store is unwritable. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
