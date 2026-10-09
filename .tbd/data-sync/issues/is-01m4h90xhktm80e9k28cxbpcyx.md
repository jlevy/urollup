---
type: is
id: is-01m4h90xhktm80e9k28cxbpcyx
title: "PR #25 A2: parity ledger records the --session gap as Claude-only"
kind: bug
status: in_progress
priority: 2
version: 2
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4h90j2wpkd56gh2rswddyj1
hold: null
hold_until: null
created_at: 2026-10-09T21:27:35.217Z
updated_at: 2026-10-09T21:27:59.099Z
started_at: 2026-10-09T21:27:59.097Z
---
Medium. tests/parity/ledger.toml:6-9. Codex has the same gap; parity shows no Codex difference only because ccusage also counts the shared response in both threads. Review: https://github.com/jlevy/urollup/pull/25#pullrequestreview-5475319334 (PR #25).
