---
type: is
id: is-01m2ksp6dstb6fwkbmy587tt8a
title: Add fixture cases the scanned session corpus lacked
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
created_at: 2026-09-16T00:25:54.862Z
updated_at: 2026-09-16T00:25:54.862Z
---
The Claude Code session scanned on 2026-09-15 (Claude Code 2.1.270) held no progress records, no advisor_message iterations, no subagents/workflows/ directory, no uuid replay and no sub-millisecond timestamp, so those rules rest on synthetic cases built from ccusage, session-report and agentfdr shapes.

- Derive real cases with scripts/sanitize-claude-fixture.mjs when a session shows any of them, and record which Claude Code version wrote it.
- Claude Code inline sidechains (older transcripts with isSidechain turns and no spawn ID, design 3.2) have no fixture at all yet; add one, synthetic if no such transcript survives the 30-day retention.
- Keep the fixtures README's case table and 'Not modeled yet' section in step.
