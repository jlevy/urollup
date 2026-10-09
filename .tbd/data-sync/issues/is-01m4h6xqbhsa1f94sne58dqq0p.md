---
type: is
id: is-01m4h6xqbhsa1f94sne58dqq0p
title: "PR #24 A3: stable sessions use the whole-history ledger, not the old per-session ledger"
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
created_at: 2026-10-09T20:50:53.423Z
updated_at: 2026-10-09T21:39:52.987Z
started_at: 2026-10-09T20:50:59.971Z
closed_at: 2026-10-09T21:39:52.986Z
close_reason: "fixed in ea78815 and 0059751: whole-history classification documented, pinned by Rust and Python tests, excluded.without_owned_requests counter; reply https://github.com/jlevy/urollup/pull/24#issuecomment-6089705866"
resolution: null
duplicate_of: null
---
Medium, PR #24 review A (https://github.com/jlevy/urollup/pull/24#pullrequestreview-5474469420). tests/parity/local_aggregate.py:184-233, tests/parity/README.md:48-52, docs/usage-analysis.md:33-34. Conflicting-owner requests become ambiguous in whole history; document, pin with a test, optionally count sessions without owned requests separately.
