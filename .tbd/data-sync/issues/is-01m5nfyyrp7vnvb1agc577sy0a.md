---
type: is
id: is-01m5nfyyrp7vnvb1agc577sy0a
title: Accept the sessions view's subagent selector in --session
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:30.517Z
updated_at: 2026-10-23T23:01:30.517Z
---
sessions --format json prints a subagent's session as <session>/<agent>, but --session <session>/<agent> fails with 'did not match any discovered session'; native_selectors (selection.rs:372-390) accepts only the bare agent ID, which becomes ambiguous once a copied subagent continues under another session. Fix: accept the printed form. Related: uro-4v0g.
