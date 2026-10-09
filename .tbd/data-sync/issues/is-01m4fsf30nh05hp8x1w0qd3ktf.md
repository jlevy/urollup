---
type: is
id: is-01m4fsf30nh05hp8x1w0qd3ktf
title: The e2e naive-sum context line double counts total_tokens
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T07:36:27.924Z
updated_at: 2026-10-09T07:36:27.924Z
---
scripts/check-e2e-results.mjs tokenTotal adds every token key except reasoning, but corpus naive rules state total_tokens alongside the categories, so the 'for context' overcount line prints doubled figures (token-usage-records, counter-reset-epoch, compaction-lowered-total) and can pick the wrong worst rule. Display only; never an assertion. Skip total_tokens (or use it alone when present) and add a test with a corpus-shaped naive rule.
