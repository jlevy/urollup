---
type: is
id: is-01m4h90z7bxrpscrfkvv27tr9t
title: "PR #25 A3: ledger entry fields omit key, bead and gap nature"
kind: bug
status: closed
priority: 2
version: 3
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4h90j2wpkd56gh2rswddyj1
hold: null
hold_until: null
created_at: 2026-10-09T21:27:36.928Z
updated_at: 2026-10-09T22:34:58.731Z
started_at: 2026-10-09T21:28:01.281Z
closed_at: 2026-10-09T22:34:58.730Z
close_reason: "fixed in 796d8f0: key pinned, uro-s71z in retirement, gap marked in comment; cause kept (no gap value in LEDGER_CAUSES); https://github.com/jlevy/urollup/pull/25#issuecomment-6090389585"
resolution: null
duplicate_of: null
---
Low. tests/parity/ledger.toml:10-46. Pin key on the rules, name uro-s71z in retirement, and mark the cause as a urollup gap if the schema allows. Review: https://github.com/jlevy/urollup/pull/25#pullrequestreview-5475319334 (PR #25).
