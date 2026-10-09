---
type: is
id: is-01m4h6xv0as69ehg96fk1gag7b
title: "PR #24 A5: check urollup version and sessions contract before the other whole-history runs"
kind: bug
status: closed
priority: 2
version: 3
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4h6x4yjxwq1jjk8rvaty17y
hold: null
hold_until: null
created_at: 2026-10-09T20:50:57.159Z
updated_at: 2026-10-09T21:39:54.106Z
started_at: 2026-10-09T20:51:01.345Z
closed_at: 2026-10-09T21:39:54.104Z
close_reason: "fixed in 0059751: version, then sessions contract, before daily and report; tests assert order and early stops; reply https://github.com/jlevy/urollup/pull/24#issuecomment-6089705866"
resolution: null
duplicate_of: null
---
Low, PR #24 review A (https://github.com/jlevy/urollup/pull/24#pullrequestreview-5474469420). tests/parity/local_aggregate.py:368-373 runs version and the sessions contract check after three ingests.
