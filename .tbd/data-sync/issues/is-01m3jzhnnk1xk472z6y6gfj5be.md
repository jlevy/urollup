---
type: is
id: is-01m3jzhnnk1xk472z6y6gfj5be
title: Organize core CLI delivery scope and merge stack
kind: task
status: in_progress
priority: 1
version: 12
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
updated_at: 2026-10-09T17:25:35.152Z
started_at: 2026-09-28T03:04:59.752Z
---
Own the frozen alpha delivery slice and merge stack: reliable per-session and whole-machine Codex/Claude usage and supported cost rollups with explicit incomplete and unpriced coverage. PR15 is the planning foundation; prior implementation PRs 4,8,10,11,12 are merged; PR14 Cursor stays independent and outside scope. Required layers: accounting correctness and CLI honesty (kpbp,gop8,oz6w), process-wide memory safety and scale validation (6pi8,z1h1,erqo), pricing context/table/matcher (381f,wuby,neii), bounded local QA and acceptance (qg1a,d36a,ky6c). Broader joint-query artifacts, reporting skill, UI, additional adapters and optional optimization are deferred by the maintainer scope freeze; their beads remain open but do not block this alpha slice. Verify actual GitHub bases, layer-specific review, full make check, end-to-end private evidence and hosted CI on published heads. Preserve historical branches and private evidence. Prepare mergeable PRs, but do not merge or publish releases.

## Notes

Frozen scope remains stable mergeable alpha: reliable per-session and whole-machine Codex/Claude usage and costs, explicit incomplete/unpriced coverage, end-to-end synthetic and private QA, manageable memory, full make check and hosted CI. PR15 at 3776b33 passed full checks and 15 CI jobs. Accounting PR16 at 3c4e9cd is published, technically reviewed with no actionable findings and all 15 hosted checks green; not merged and no independent GitHub approval recorded. Pricing implementation remains uncommitted on codex/alpha-pricing-context; workspace tests and clippy passed, but rate policy, CLI integration, remaining correctness work, memory/scale gates and full-history acceptance are outstanding. Systematic format contracts and incident documentation are tracked under child uro-hlas. Broader artifacts, skill/UI, adapters and optional optimizations remain deferred. No merge or release publication is authorized.


The parent of this bead is:
---
type: is
id: is-01m2yrezrf530kbz15erhh7hw6
title: "Spec: Clean whole-history usage analysis and list-price estimates"
kind: epic
status: open
priority: PNaN
version: 10
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
# Blocked by: uro-wuby, uro-d36a, uro-neii, uro-ky6c, uro-oz6w, uro-qg1a, uro-381f, uro-6pi8, uro-z1h1, uro-gop8, uro-kpbp, uro-v1c9, uro-5nkv, uro-t4l1
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2yrfm4816gxgm14jk3c0erq
  - is-01m2yrgbar7pmmncksknephs5s
  - is-01m2yrgc3tern0t301ba3hhwg2
  - is-01m2yrgcxaf3ewkzv9cjawjf18
  - is-01m2yrhe5crqpp22w45b3243ev
  - is-01m2yrherhddnx2j22g3xtqedr
  - is-01m3jwwpp0h7j5azt28c75zcsj
  - is-01m3jzhnnk1xk472z6y6gfj5be
  - is-01m3na5hvvbrsrgtrn2v6w4jye
created_at: 2026-09-20T06:36:08.846Z
updated_at: 2026-09-29T00:48:51.578Z
---
Deliver G5 in the linked plan: one reconciled snapshot, reusable artifacts, joint calendar/agent/provider/model grouping, cache metrics, tools and time, and explicit list-price estimates. Reuse existing summary/bundle and pricing work. Acceptance requires documented commands and a maintained runner without bespoke scripts. Full workflow remains milestone 0.5; this does not silently expand the 0.1 release scope.


2026-10-08 PR landscape and handoff. Formal GitHub stack #17 is #15 (docs, base main) then #16 (Codex fork double-count fix, base codex/usage-analysis-workflow-plan); both heads (3776b33, 3c4e9cd) are MERGEABLE/CLEAN with 15/15 checks green, but those runs predate GHSA-vfj7-8cjw-p6xm, so a re-run of either fails npm audit until rebased onto a main that has uro-t4l1. PR #18 (fix/compressed-sources-codex-usage, base main) is standalone by design: gzip sources (uro-nc34), vanished-source handling (uro-2abk), source-incomplete coverage (uro-1h5s), subagent model lookup (uro-5nkv), lowered-total overcount (uro-v1c9) and tryscript 0.3.0 (uro-t4l1). Its commits apply cleanly on main and on #16; on #16 the workspace tests, 239 goldens and 29 fixture result checks pass, so merge order is free. PR #14 (Cursor plan) is outside the alpha slice. No PR has a recorded review on GitHub, so none meets the pr-review-requirements=standard merge gate yet; no merge is authorized. Real-log QA ran on the maintainer's machine with #16 underneath #18; values stay local. Pricing work for uro-381f/uro-neii remains uncommitted in the main checkout on codex/alpha-pricing-context (based on #16). Next: senior reviews of #15, #16 and #18 at pinned heads, user-authorized merges, then the remaining alpha gates.


The parent of this bead is:
---
type: is
id: is-01m2yrezrf530kbz15erhh7hw6
title: "Spec: Clean whole-history usage analysis and list-price estimates"
kind: epic
status: open
priority: P1
version: 10
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2yrfm4816gxgm14jk3c0erq
  - is-01m2yrgbar7pmmncksknephs5s
  - is-01m2yrgc3tern0t301ba3hhwg2
  - is-01m2yrgcxaf3ewkzv9cjawjf18
  - is-01m2yrhe5crqpp22w45b3243ev
  - is-01m2yrherhddnx2j22g3xtqedr
  - is-01m3jwwpp0h7j5azt28c75zcsj
  - is-01m3jzhnnk1xk472z6y6gfj5be
  - is-01m3na5hvvbrsrgtrn2v6w4jye
created_at: 2026-09-20T06:36:08.846Z
updated_at: 2026-09-29T00:48:51.578Z
---
Deliver G5 in the linked plan: one reconciled snapshot, reusable artifacts, joint calendar/agent/provider/model grouping, cache metrics, tools and time, and explicit list-price estimates. Reuse existing summary/bundle and pricing work. Acceptance requires documented commands and a maintained runner without bespoke scripts. Full workflow remains milestone 0.5; this does not silently expand the 0.1 release scope.


2026-10-09 progress. Merged to main at 98f9428 (merge commits, via gh stack merge of stack #23): PR #18 (gzip sources, vanished/unreadable-source coverage with source-incomplete and unreadable-twin, Codex subagent turn lookup, lowered-total rule, tryscript 0.3.0) and PR #22 (per-request model/effort e2e checks, compaction-lowered-total fixture), after senior, correctness, security and follow-up reviews with every finding dispositioned. Stack #17 restacked onto that main: #15 docs at ce226d5 (review A addressed, status refreshed), #16 at ba4d379 (reviews A/B round 1, C round 2, D focused restack review; copy-only coverage rule reverted per the user's decision with policy deferred to uro-xpd0; unverifiable fork boundaries degrade per rollout). The user authorized merging #15 and #16 once the gate passes. Draft #19 (pricing) needs a restack onto main after #16 merges. Stack #21 (#14 Cursor plan, addressed; #20 Cursor draft) is on hold by the user's decision. Open follow-ups from reviews: uro-pny9, uro-zd3m, uro-h6m9, uro-loml, uro-x3sk, uro-h2sf, uro-omlf, uro-akgu, uro-xpd0. Next: merge #15+#16, restack #19, full real-history test of merged main.
