---
type: is
id: is-01m4fsy8wjtz17e53bkhgpqqpm
title: "PR #14 A12: research brief copies non-built-in subagent type values"
kind: bug
status: closed
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsxxjgrahm03vd1fr1vhms
hold: null
hold_until: null
created_at: 2026-10-09T07:44:45.457Z
updated_at: 2026-10-09T07:50:33.895Z
started_at: 2026-10-09T07:45:12.167Z
closed_at: 2026-10-09T07:50:33.894Z
close_reason: "Fixed in f1e4277: the brief keeps Cursor's built-in subagent type names and describes the rest as user-defined types; removed values are not repeated anywhere. Confirmed by git grep over docs (no matches for the removed values), flowmark --check and the link check."
resolution: null
duplicate_of: null
---
Severity: Low (privacy). PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: docs/project/research/research-2026-09-19-cursor-agent-logs.md :168-170 (subagent paragraph).

Problem: the subagentTypeName value list mixed Cursor built-in types with values that look like user-defined agent names read from local stores. The product plan allows committed aggregates, never log content. Values are not repeated here.

Fix: keep the built-in type names and describe the rest generically as user-defined types.
