---
type: is
id: is-01m4fsqqgrygh90wfq4tfvprzn
title: "PR #15 A4: QA step 6.2 leaves raw transcript copies in the preserved evidence directory"
kind: bug
status: in_progress
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsq5hrr2dsczqgy5bxcnv1
hold: null
hold_until: null
created_at: 2026-10-09T07:41:11.063Z
updated_at: 2026-10-09T07:48:30.979Z
started_at: 2026-10-09T07:48:30.978Z
---
tests/qa/full-history-rollup.qa.md:440-442 with :107-111,534-537. work=$(mktemp -d "$QA/resume.XXXX") copies full private transcripts into $QA, which step 8.2 now preserves. Use disposable scratch and remove it after 6.2. Severity: Medium. PR #15, review A (https://github.com/jlevy/urollup/pull/15#pullrequestreview-5467133135), pinned head 3776b333aa08e24b3bc0a9f4d4ce29e49b44fc58.
