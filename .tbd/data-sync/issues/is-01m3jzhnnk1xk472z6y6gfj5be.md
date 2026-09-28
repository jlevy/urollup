---
type: is
id: is-01m3jzhnnk1xk472z6y6gfj5be
title: Organize core CLI delivery scope and merge stack
kind: task
status: in_progress
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2yrezrf530kbz15erhh7hw6
hold: null
hold_until: null
created_at: 2026-09-28T03:04:45.490Z
updated_at: 2026-09-28T03:13:02.893Z
started_at: 2026-09-28T03:04:59.752Z
---
Own the core CLI delivery slice and current PR inventory. PR15 is the plan foundation; prior implementation PRs 4,8,10,11,12 are merged; PR14 Cursor is independent and outside scope. Plan runtime layers for accounting correctness (kpbp,gop8,oz6w), process-wide safety and scale (6pi8,z1h1,erqo), joint queries and existing artifacts (qvp1,ni7m,vgea,cye3,x8r8), bounded QA and reporting skill (qg1a,jpmf,d36a,ky6c). Verify actual GitHub stack bases, layer-specific checks and published-head CI. Preserve historical branches and private evidence; do not merge or publish. Completion requires the mapped plan and actual implemented layers, not only a proposed stack.

## Notes

Planning diff review: no blocking findings. Reuses existing accounting, query and artifact owners; does not add another engine or relax runtime gates. Verified all hosted PR states, both open PR heads/checks, lack of formal reviews and overlapping design/product-plan files. Cursor remains independent. Local gh-stack now tracks main -> codex/usage-analysis-workflow-plan; no runtime PR exists yet. Full make check is running on the external-disk test copy; final five-line prose addition passed targeted flowmark and diff checks. Completion remains open until real delivery layers are implemented, checked and linked.
