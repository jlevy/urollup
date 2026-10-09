---
type: is
id: is-01m4fdr8epn84nfkrtbxsd1txr
title: Document that Claude Code deletes transcripts after cleanupPeriodDays
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T04:11:45.493Z
updated_at: 2026-10-09T04:11:45.493Z
---
Claude Code removes project transcripts older than cleanupPeriodDays (default 30) at startup, compressed or not, so a whole-history Claude report covers about a month unless the user raises that setting or archives transcripts elsewhere. Codex keeps archived_sessions. Say so in docs/usage-analysis.md and the full-history QA playbook, and suggest raising cleanupPeriodDays or archiving transcripts to a root passed with --source, before users treat 'all' as all-time.
