---
type: is
id: is-01m4fsy9zy17r0p6mpa9hrp37t
title: "PR #14 A15: say which Fable spellings stay unknown"
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
created_at: 2026-10-09T07:44:46.589Z
updated_at: 2026-10-09T07:50:34.536Z
started_at: 2026-10-09T07:45:13.135Z
closed_at: 2026-10-09T07:50:34.535Z
close_reason: "Fixed in 712016e: the Goals provider bullet and the Facet Contract provider row say a claude-fable-* value maps to anthropic through the claude-* rule and only a Fable spelling without that prefix stays unknown; the model bullet keeps Fable's picker-to-catalog mapping open until a stored value is seen. Confirmed by pinned flowmark --auto --check . (exit 0), git diff --check (clean), and the relative link and anchor check of the four changed files (907 links, bad=0)."
resolution: null
duplicate_of: null
---
Severity: Low (suggestion). PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: plan-2026-09-19-cursor-dialect.md :53 (Goals model bullet), :206 (Facet Contract provider row).

Suggestion: a stored claude-fable-* value already maps to provider anthropic through the claude-* rule, so "Fable stays unknown until seen" applies only to spellings without that prefix. Say so explicitly.
