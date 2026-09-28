---
type: is
id: is-01m3jzhnnk1xk472z6y6gfj5be
title: Organize core CLI delivery scope and merge stack
kind: task
status: in_progress
priority: 1
version: 7
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2yrezrf530kbz15erhh7hw6
hold: null
hold_until: null
created_at: 2026-09-28T03:04:45.490Z
updated_at: 2026-09-28T03:45:40.345Z
started_at: 2026-09-28T03:04:59.752Z
---
Own the core CLI delivery slice and current PR inventory. PR15 is the plan foundation; prior implementation PRs 4,8,10,11,12 are merged; PR14 Cursor is independent and outside scope. Plan runtime layers for accounting correctness (kpbp,gop8,oz6w), process-wide safety and scale (6pi8,z1h1,erqo), joint queries and existing artifacts (qvp1,ni7m,vgea,cye3,x8r8), bounded QA and reporting skill (qg1a,jpmf,d36a,ky6c). Verify actual GitHub stack bases, layer-specific checks and published-head CI. Preserve historical branches and private evidence; do not merge or publish. Completion requires the mapped plan and actual implemented layers, not only a proposed stack.

## Notes

Scope superseded by explicit maintainer direction: finish a stable mergeable alpha, not further improvements. Required: reliable per-session and whole-machine Codex/Claude rollups including costs, explicit incomplete/unpriced coverage, end-to-end synthetic and private real-history tests, manageable memory, full make check and hosted CI on each published stack head. Pricing work is required despite its older milestone number; reuse uro-381f and uro-neii with existing pricing prerequisites. Defer broader artifact/export workflow, reporting skill, UI, extra adapters and optional optimizations unless strictly necessary for these acceptance cases. PR15 at 3776b33 has all 15 CI jobs green and full make check passed. Runtime accounting changes remain uncommitted on codex/codex-fork-accounting; not merge-ready. No merge or release publication is authorized.
