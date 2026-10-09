---
type: is
id: is-01m4fsqqwj8dz63y1qz3sh2gkx
title: "PR #15 A5: full-history QA playbook hard-codes one host's scratch volume"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsq5hrr2dsczqgy5bxcnv1
hold: null
hold_until: null
created_at: 2026-10-09T07:41:11.441Z
updated_at: 2026-10-09T07:57:01.776Z
started_at: 2026-10-09T07:48:31.321Z
closed_at: 2026-10-09T07:57:01.775Z
close_reason: "Fixed on PR #15 branch (head e17d47811bc12173ab11c5aa8b00ad35df0b4150) in eb669a4: qa_setup with QA_SCRATCH_ROOT and return 1; exercised in a scratch shell (unset, missing, unwritable, fake-mounted, override cases). Disposition reply: https://github.com/jlevy/urollup/pull/15#issuecomment-6076871120"
resolution: null
duplicate_of: null
---
tests/qa/full-history-rollup.qa.md:136-139 against :101-103 hard-code /Volumes/spud-ext1 and use '|| exit 1', which closes an interactive shell. Parameterize the volume (QA_SCRATCH_ROOT) and guard without exit. Severity: Medium. PR #15, review A (https://github.com/jlevy/urollup/pull/15#pullrequestreview-5467133135), pinned head 3776b333aa08e24b3bc0a9f4d4ce29e49b44fc58.
