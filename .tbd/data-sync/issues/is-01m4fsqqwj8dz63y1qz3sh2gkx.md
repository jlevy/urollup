---
type: is
id: is-01m4fsqqwj8dz63y1qz3sh2gkx
title: "PR #15 A5: full-history QA playbook hard-codes one host's scratch volume"
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
created_at: 2026-10-09T07:41:11.441Z
updated_at: 2026-10-09T07:48:31.321Z
started_at: 2026-10-09T07:48:31.321Z
---
tests/qa/full-history-rollup.qa.md:136-139 against :101-103 hard-code /Volumes/spud-ext1 and use '|| exit 1', which closes an interactive shell. Parameterize the volume (QA_SCRATCH_ROOT) and guard without exit. Severity: Medium. PR #15, review A (https://github.com/jlevy/urollup/pull/15#pullrequestreview-5467133135), pinned head 3776b333aa08e24b3bc0a9f4d4ce29e49b44fc58.
