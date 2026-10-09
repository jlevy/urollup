---
type: is
id: is-01m4fsqps65kr404wmr7b5vnjt
title: "PR #15 A2: governing review doc contradicts itself and goes stale when the stack lands"
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
created_at: 2026-10-09T07:41:10.309Z
updated_at: 2026-10-09T07:48:30.276Z
started_at: 2026-10-09T07:48:30.274Z
---
docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md:19,30-35,48,108,112. Step 4 still says land #4..#12; :112 contradicts :48; Current PR Landscape records #15 at superseded head 04a3fdd and 'neither has a formal review'; :48-50 omits the copied 100-token prefix. Severity: Medium. PR #15, review A (https://github.com/jlevy/urollup/pull/15#pullrequestreview-5467133135), pinned head 3776b333aa08e24b3bc0a9f4d4ce29e49b44fc58.
