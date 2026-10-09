---
type: is
id: is-01m4fdr8epn84nfkrtbxsd1txr
title: Document that Claude Code deletes transcripts after cleanupPeriodDays
kind: task
status: closed
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-10-09T04:11:45.493Z
updated_at: 2026-10-09T17:33:07.513Z
started_at: 2026-10-09T07:48:34.151Z
closed_at: 2026-10-09T17:33:07.512Z
close_reason: "Merged to main in bdaa7ce via PRs #15 and #16 (stack #17) after senior, correctness, follow-up and restack reviews with every finding dispositioned; real-history run on the merged tree verified."
resolution: null
duplicate_of: null
---
Claude Code removes project transcripts older than cleanupPeriodDays (default 30) at startup, compressed or not, so a whole-history Claude report covers about a month unless the user raises that setting or archives transcripts elsewhere. Codex keeps archived_sessions. Say so in docs/usage-analysis.md and the full-history QA playbook, and suggest raising cleanupPeriodDays or archiving transcripts to a root passed with --source, before users treat 'all' as all-time.

## Notes

Documented on PR #15 (head e17d47811bc12173ab11c5aa8b00ad35df0b4150) in e17d478 (docs/usage-analysis.md) and eb669a4 (tests/qa/full-history-rollup.qa.md prerequisite and step 1.2 retention check). Close when PR #15 merges. Disposition reply: https://github.com/jlevy/urollup/pull/15#issuecomment-6076871120
