---
type: is
id: is-01m3jzhnnk1xk472z6y6gfj5be
title: Organize core CLI delivery scope and merge stack
kind: task
status: in_progress
priority: 1
version: 10
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2yrezrf530kbz15erhh7hw6
child_order_hints:
  - is-01m3n7xr89n088q0zf425zzj2y
hold: null
hold_until: null
created_at: 2026-09-28T03:04:45.490Z
updated_at: 2026-09-29T00:17:08.982Z
started_at: 2026-09-28T03:04:59.752Z
---
Own the frozen alpha delivery slice and merge stack: reliable per-session and whole-machine Codex/Claude usage and supported cost rollups with explicit incomplete and unpriced coverage. PR15 is the planning foundation; prior implementation PRs 4,8,10,11,12 are merged; PR14 Cursor stays independent and outside scope. Required layers: accounting correctness and CLI honesty (kpbp,gop8,oz6w), process-wide memory safety and scale validation (6pi8,z1h1,erqo), pricing context/table/matcher (381f,wuby,neii), bounded local QA and acceptance (qg1a,d36a,ky6c). Broader joint-query artifacts, reporting skill, UI, additional adapters and optional optimization are deferred by the maintainer scope freeze; their beads remain open but do not block this alpha slice. Verify actual GitHub bases, layer-specific review, full make check, end-to-end private evidence and hosted CI on published heads. Preserve historical branches and private evidence. Prepare mergeable PRs, but do not merge or publish releases.

## Notes

Frozen scope remains stable mergeable alpha: reliable per-session and whole-machine Codex/Claude usage and costs, explicit incomplete/unpriced coverage, end-to-end synthetic and private QA, manageable memory, full make check and hosted CI. PR15 at 3776b33 passed full checks and 15 CI jobs. Accounting PR16 at 3c4e9cd is published, technically reviewed with no actionable findings and all 15 hosted checks green; not merged and no independent GitHub approval recorded. Pricing implementation remains uncommitted on codex/alpha-pricing-context; workspace tests and clippy passed, but rate policy, CLI integration, remaining correctness work, memory/scale gates and full-history acceptance are outstanding. Systematic format contracts and incident documentation are tracked under child uro-hlas. Broader artifacts, skill/UI, adapters and optional optimizations remain deferred. No merge or release publication is authorized.
