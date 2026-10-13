---
type: is
id: is-01m4p6rbyw6p63jbzgnrerqv5q
title: A Codex child's own step equal to the parent's next step is dropped as a copy
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-11T19:24:10.063Z
updated_at: 2026-10-13T07:42:36.393Z
closed_at: 2026-10-13T07:42:36.392Z
close_reason: "Duplicate of uro-uwv0 (same PR #33 review A4 finding), which main's docs cite."
resolution: null
duplicate_of: null
---
Found in PR #33 review A (A4), shared with PR #32's turn inference. When a migrated child's (subagent or user fork) only own counter step exactly matches the parent's next counter step on all six counts, the contradiction check treats it as the parent's copy and drops it silently with coverage complete (synthetic: 160 against a real 220). PR #33 pins both shapes (P4b fork, P4c migrated subagent). Fix direction: exclude the counter that voids an inferred start as undecided (a codex-history-boundary-unverified gap) rather than as a copy; this raises PR #32's C3 undecided count by one.
