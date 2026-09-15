---
type: is
id: is-01m2hsnpx0am8stcjg0vy3fb2r
title: "PR #2 review R3: Deny-list strip policy can leak content under unknown keys"
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
created_at: 2026-09-15T05:47:10.111Z
updated_at: 2026-09-15T16:28:29.849Z
started_at: 2026-09-15T16:13:57.611Z
closed_at: 2026-09-15T16:28:29.848Z
close_reason: "Fixed in 2b6f811 (maintainer decision): versioned capture policy for the local store stubs known content and keeps unknown keys verbatim with diagnostics; strict per-dialect export allow-list for summaries and bundles with {type, bytes} stubs; leak fixture and no-echo diagnostics test added."
resolution: null
duplicate_of: null
---
arch strip policy enumerates content to stub but not the rule for unknown keys; the spike keeps short strings under unlisted keys. Invert to a per-dialect allow-list, stub every other value, version it, add an unknown-key leak fixture, and test that diagnostics never echo redacted values. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
