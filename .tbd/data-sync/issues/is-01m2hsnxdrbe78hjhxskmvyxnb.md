---
type: is
id: is-01m2hsnxdrbe78hjhxskmvyxnb
title: "PR #2 review S2: Suggestion: external ground-truth check before 1.0"
kind: bug
status: closed
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:16.791Z
updated_at: 2026-09-15T16:28:32.723Z
started_at: 2026-09-15T16:13:57.806Z
closed_at: 2026-09-15T16:28:32.722Z
close_reason: "Fixed in 2b6f811: Testing Strategy adds a pre-1.0 ground-truth check of totals against a provider usage export for one consented corpus."
resolution: null
duplicate_of: null
---
Check one consented corpus against a provider usage export (Anthropic Console or OpenAI usage) before 1.0, at least for totals, beyond internal agreement and ccusage/agentfdr divergences. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
