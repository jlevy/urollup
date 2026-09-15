---
type: is
id: is-01m2hsnsqwtsc4ekt4kj116qz0
title: "PR #2 review R7: Per-row JSON Schema validation heavier than needed"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:13.016Z
updated_at: 2026-09-15T16:28:31.581Z
started_at: 2026-09-15T16:13:57.708Z
closed_at: 2026-09-15T16:28:31.580Z
close_reason: "Fixed in 2b6f811: typed serde structs with deny_unknown_fields validate on read and write; JSON Schema validation runs in urollup validate and tests proving serde, compiled schema and softschema agree."
resolution: null
duplicate_of: null
---
Validating every table row against compiled JSON Schema on read and write duplicates typed serde deny_unknown_fields. Make typed serde the hot-path validator; run JSON Schema in urollup validate and tests proving serde and softschema agree on every fixture. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
