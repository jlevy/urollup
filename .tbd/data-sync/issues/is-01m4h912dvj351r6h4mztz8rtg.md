---
type: is
id: is-01m4h912dvj351r6h4mztz8rtg
title: "PR #25 A5: SelectedRequest doubles in size for every command"
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
created_at: 2026-10-09T21:27:40.107Z
updated_at: 2026-10-09T21:28:04.361Z
started_at: 2026-10-09T21:28:04.358Z
---
Low (suggestion). crates/urollup-core/src/query/aggregate.rs:332-360. Only sessions needs the source agent. Review: https://github.com/jlevy/urollup/pull/25#pullrequestreview-5475319334 (PR #25).
