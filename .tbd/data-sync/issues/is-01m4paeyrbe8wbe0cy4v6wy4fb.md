---
type: is
id: is-01m4paeyrbe8wbe0cy4v6wy4fb
title: Thread-less Codex settings snapshots never set a migrated (Unmarked) fork's pricing tier
kind: bug
status: open
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2kswxt5gxe3s49wch9dss46
  - type: blocks
    target: is-01m5nfwr7vgz75j01zt8p449kk
parent_id: is-01m2yrgc3tern0t301ba3hhwg2
created_at: 2026-10-11T20:28:55.947Z
updated_at: 2026-10-23T23:02:38.558Z
---
Found in PR #33 review B (M3, merge guidance for PR #19). PR #19 attributes a thread_settings_applied snapshot that names no thread with 'None if before_boundary || native_boundary.cannot_place(...)'. PR #33's NativeBoundary::Unmarked lies past a migrated user fork's last line (and PR #32's migrated subagents have a boundary past their end), so that condition holds on every line and such a snapshot never sets the fork's own service tier or provider: a probe on the trial merge gives the fork's own requests (20, no tier) where (20, priority) is right. Totals are unaffected; nothing consumes the tier until the price matcher (uro-neii) lands. PR #19's merge keeps the conservative rule, corrects the docs and pins the no-tier result. Proposed rule, consistent with PR #33's A5: a thread-less snapshot inside an open inferred start sets the rollout's own thread; before the boundary with no start open, hold it pending and apply it when the next turn opens the child's own lines, dropping it if the next turn is one the parent recorded; save the pricing context in InferredStart and restore it when the start is voided. Test: an_unmarked_forks_thread_less_settings_set_its_tier_only_from_its_own_turns for migrated_fork_meta() and migrated_meta(12), with N1 (20, priority)/(15, none), N2 (20, none)/(15, priority), all 195 complete, parent (100, flex)/(60, flex), plus a voided variant. Should land before the matcher reads ThreadSetting tiers.
