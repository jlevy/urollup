---
type: is
id: is-01m4fsqtgr0fy69a62pt1dd9cx
title: "PR #15 A12: say directly that account grouping yields only the unknown group"
kind: bug
status: in_progress
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsq5hrr2dsczqgy5bxcnv1
hold: null
hold_until: null
created_at: 2026-10-09T07:41:14.135Z
updated_at: 2026-10-09T07:48:33.828Z
started_at: 2026-10-09T07:48:33.827Z
---
Suggestion. docs/usage-analysis.md:57 'account breakdown is currently unknown' should say report --group-by account is accepted but every row is unknown because no adapter records an account (query/aggregate.rs:390-391). Severity: Low. PR #15, review A (https://github.com/jlevy/urollup/pull/15#pullrequestreview-5467133135), pinned head 3776b333aa08e24b3bc0a9f4d4ce29e49b44fc58.
