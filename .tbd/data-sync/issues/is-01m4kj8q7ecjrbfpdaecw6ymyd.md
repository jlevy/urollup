---
type: is
id: is-01m4kj8q7ecjrbfpdaecw6ymyd
title: Codex child counts a rolled-back parent turn's usage again
kind: bug
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-10T18:47:36.940Z
updated_at: 2026-10-10T20:41:59.081Z
---
Found in PR #32 review C (C2). When a Codex root rolls back a turn that spawned a subagent, Codex's rollback plan drops that turn's turn_context and counters (including the estimate counter before the marker) from the root, while the root's next counter step still includes the dropped usage. A child that copied the dropped turn then cannot match it against the parent's turns, so turn-ID copied-history inference treats it as the child's own and counts it twice (synthetic: 380 against 330; two siblings 460 against 360), coverage complete. Pre-existing on main for unmigrated legacy children; PR #32 extends the inference to migrated children. A local real-history check found no inferred-own totals inside a parent rollback window. Fix direction: detect the parent's rollback signature (a counter step whose delta exceeds its last usage) and report a gap for child regions whose totals fall inside it.

## Notes

PR #32 review E (E3): the rollback double count also survives in a counter-only child whose fork settings event names it (Codex 0.152+ writes that settings event right after the copied prefix), because a line naming the child ends the inferred-start window without voiding it; any inferred start before that settings event is a turn the parent lost. Synthetic: r2n plus settings(CHILD) before c1 gives 380 against a real 330. A fix likely needs gating on the child's creating cli_version, or treating an inferred start before the child's own fork settings event as unverifiable.
