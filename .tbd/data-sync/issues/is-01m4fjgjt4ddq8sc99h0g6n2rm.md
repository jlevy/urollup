---
type: is
id: is-01m4fjgjt4ddq8sc99h0g6n2rm
title: Check per-request model and effort in e2e fixture results
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T05:34:56.834Z
updated_at: 2026-10-09T05:34:56.834Z
---
scripts/check-e2e-results.mjs compares totals, ownership, copies, limits and diagnostics, but not each request's model and effort, although every expected.json records them. The archived-rename and auto-review-model goldens recorded model unknown for subagent and guardian requests whose expected.json names gpt-5.2-codex and codex-auto-review, and make e2e-results still passed (uro-5nkv). Compare model, model_basis and effort per request so a lost turn context fails the results check, not only the transcript golden.
