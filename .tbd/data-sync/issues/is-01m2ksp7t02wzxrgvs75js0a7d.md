---
type: is
id: is-01m2ksp7t02wzxrgvs75js0a7d
title: Verify the unverified Claude transcript shapes in the fixtures
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - fixtures
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:56.272Z
updated_at: 2026-09-16T00:25:56.272Z
---
Two shapes in crates/urollup-core/tests/fixtures/claude-project/ are modeled, not observed, and are flagged in their cases:

- The /btw side-question transcript's file name, agent-aaside_question-<hex16>.jsonl, combines ccusage's note that /btw logs live under subagents/ with session-report's labeled-agent convention (agent-a<label>-<hex>). Confirm the label, or the whole naming, against a real side-question transcript.
- claude-project/missing-request-id models Bedrock-style msg_bdrk_ message IDs without requestId; which gateways and providers omit requestId, and what their message IDs look like, is unverified.

Fix the fixtures and the fixtures README when either is settled.
