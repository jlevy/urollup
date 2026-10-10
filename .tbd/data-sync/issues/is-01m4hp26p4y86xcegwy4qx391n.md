---
type: is
id: is-01m4hp26p4y86xcegwy4qx391n
title: Session selection drops request-scoped diagnostics
kind: bug
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-10T01:15:28.766Z
updated_at: 2026-10-10T01:15:46.674Z
---
Found in PR #25 review B (B1): under any --session, query::aggregate::diagnostic_summaries keeps only diagnostics whose subject is a selected thread, but several codes take a request ID as their subject: conflicting-owners, conflicting-models, revision-disagreement / claude-block-usage-conflict, copy-without-original / claude-nested-copy-without-original and unresolved-candidate. report, daily and sessions therefore drop them under any --session, even when every top-level session is selected (fixtures ambiguous-owner in both dialects, block-record-selection, progress-nested-subagent). Totals are unaffected. Fix needs a rule mapping a request-scoped diagnostic to a selection (for example: keep it when any selected session owns or claims the request), overlapping uro-s71z; update all three commands' goldens.
