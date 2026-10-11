---
type: is
id: is-01m4mxfv8ss1bh2zwn29ar2mwp
title: Native counter-only Codex forks count their parent's seeded total again
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-11T07:22:59.210Z
updated_at: 2026-10-11T07:22:59.210Z
---
Found while fixing uro-p9ua (PR #33), pre-existing on main. A native paginated Codex fork written by 0.150-0.152 declares history_base, copies no parent lines, and has Codex seed its running total from the parent; when the fork is counter-only, its first counter step's total includes the parent's usage, and urollup counts that seeded total as the fork's own (synthetic: 180 where 20 is correct), coverage complete. PR #33 pins main's behavior in a_counter_only_native_fork_still_counts_its_seeded_total. Fix direction: check a counter-only fork's first total against the parent's reported totals and treat the seed as inherited (first-step check), reporting a gap when the parent is absent. Needs a real-history count of native counter-only forks with history_base from 0.150-0.152.
