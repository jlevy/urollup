---
type: is
id: is-01m4fsy85gy08wcjrw8ba58q59
title: "PR #14 A10: Cursor pricing omits billing channel and label mapping"
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
created_at: 2026-10-09T07:44:44.719Z
updated_at: 2026-10-09T07:50:33.139Z
started_at: 2026-10-09T07:45:11.528Z
closed_at: 2026-10-09T07:50:33.138Z
close_reason: "Fixed in 712016e: pricing matches on provider, billing channel and model; Cursor rows carry billing channel cursor; a first-party API list price is an estimate for that channel; the versioned label mapping (e.g. claude-4.5-opus-high-thinking -> claude-opus-4-5) is an input to the matcher. Confirmed by pinned flowmark --auto --check . (exit 0), git diff --check (clean), and the relative link and anchor check of the four changed files (907 links, bad=0)."
resolution: null
duplicate_of: null
---
Severity: Low. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: plan-2026-09-19-cursor-dialect.md :352-355 (pricing paragraph); docs/urollup-design.md :1233 (§4.5 rate row keys on billing channel).

Problem: the plan says pricing matches on provider plus model, but the rate row also keys on billing channel, and Cursor is a distinct billing channel (Cursor-hosted models versus third-party models at API rates). The versioned picker-label mapping (for example claude-4.5-opus-high-thinking -> claude-opus-4-5) is not named as a pricing input.

Fix: state that Cursor rows carry billing channel cursor, that first-party API list prices are an estimate for that channel, and that the versioned label mapping is an input to the price matcher.
