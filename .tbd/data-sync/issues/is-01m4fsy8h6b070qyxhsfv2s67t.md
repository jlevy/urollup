---
type: is
id: is-01m4fsy8h6b070qyxhsfv2s67t
title: "PR #14 A11: generic agent and provider facets belong to uro-qvp1"
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
created_at: 2026-10-09T07:44:45.093Z
updated_at: 2026-10-09T07:50:33.507Z
started_at: 2026-10-09T07:45:11.850Z
closed_at: 2026-10-09T07:50:33.506Z
close_reason: "Fixed in 712016e: the plan says the generic agent and provider facets, versioned mappings and separate billing channel belong to uro-qvp1; this plan contributes the Cursor catalog-family mapping table and fixtures; the Phase 1 facet item uses the uro-qvp1 facets. Confirmed by pinned flowmark --auto --check . (exit 0), git diff --check (clean), and the relative link and anchor check of the four changed files (907 links, bad=0)."
resolution: null
duplicate_of: null
---
Severity: Low. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: plan-2026-09-19-cursor-dialect.md :225-228, :382-383.

Problem: the plan makes request-level provider and --group-by provider / agent part of its own Phase 1, while the tracker assigns the generic agent and provider facets (versioned mappings, separate billing channel) to uro-qvp1 and blocks uro-2qxq on it.

Fix: say the generic facets come from uro-qvp1 and that this plan contributes only the Cursor mapping table and fixtures.
