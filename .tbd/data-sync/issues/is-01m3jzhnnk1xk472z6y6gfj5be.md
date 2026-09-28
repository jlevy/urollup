---
type: is
id: is-01m3jzhnnk1xk472z6y6gfj5be
title: Organize core CLI delivery scope and merge stack
kind: task
status: in_progress
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2yrezrf530kbz15erhh7hw6
hold: null
hold_until: null
created_at: 2026-09-28T03:04:45.490Z
updated_at: 2026-09-28T03:19:30.197Z
started_at: 2026-09-28T03:04:59.752Z
---
Own the core CLI delivery slice and current PR inventory. PR15 is the plan foundation; prior implementation PRs 4,8,10,11,12 are merged; PR14 Cursor is independent and outside scope. Plan runtime layers for accounting correctness (kpbp,gop8,oz6w), process-wide safety and scale (6pi8,z1h1,erqo), joint queries and existing artifacts (qvp1,ni7m,vgea,cye3,x8r8), bounded QA and reporting skill (qg1a,jpmf,d36a,ky6c). Verify actual GitHub stack bases, layer-specific checks and published-head CI. Preserve historical branches and private evidence; do not merge or publish. Completion requires the mapped plan and actual implemented layers, not only a proposed stack.

## Notes

Planning layer committed and pushed as 3776b33 on PR15. Full make check passed, including all 30 negative gate probes; final review-status prose passed flowmark and diff checks. CI for 3776b33 is pending. Actual local stack is main -> codex/usage-analysis-workflow-plan -> codex/codex-fork-accounting; runtime layer has no PR yet. No merges or deletions. Accounting regression is being implemented under uro-kpbp.
