---
type: is
id: is-01m4fwh3f769xec24nepn69azv
title: Compare tokens per model and effort bucket in e2e results
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T08:29:59.654Z
updated_at: 2026-10-09T08:29:59.654Z
---
Follow-up to uro-dp4x from PR #22 review A (A4): the models and efforts results compare request counts per value, so two requests with swapped models, or an advisor component's tokens charged to the main model, still pass. Expected rows already carry tokens, and advisor-iterations matches the golden's model rows today. Compare per-bucket token categories as well, with tests for both shapes.
