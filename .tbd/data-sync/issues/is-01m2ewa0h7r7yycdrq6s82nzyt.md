---
type: is
id: is-01m2ewa0h7r7yycdrq6s82nzyt
title: "Plan: make performance targets measurable"
kind: task
status: closed
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe0xffh11dgc6acjznk3r
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:29.190Z
updated_at: 2026-09-14T02:51:47.717Z
started_at: 2026-09-14T02:38:24.497Z
closed_at: 2026-09-14T02:51:47.716Z
close_reason: Performance targets table with seeded synthetic corpora (~64 MiB, 1 GiB), reference hardware, CI merge-base comparison on ubuntu-24.04, run counts, results location and 10% regression policy.
resolution: null
duplicate_of: null
---
Performance gates reference an undocumented 'reference laptop', and the 200 ms resident query target has no corpus size. Done when the plan defines reference hardware (or a CI benchmark environment), corpus size and composition for each target, and how benchmarks are run and recorded.
