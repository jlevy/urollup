---
type: is
id: is-01m4fzve52xw6hsv2z7f2ye5xq
title: "PR #16 C1: first-step check skips an explicit subagent_history_start_ordinal of 0"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fzvd803vx5er2c7k9cycgn
created_at: 2026-10-09T09:28:03.998Z
updated_at: 2026-10-09T09:47:40.152Z
closed_at: 2026-10-09T09:47:40.150Z
close_reason: "fixed in f456852 (PR #16 head 1bec262): first-step check applies to every explicit boundary including 0; confirmed by an_explicit_boundary_of_zero_still_checks_the_first_counter_step (red at 5d8cdeb: [120, 100]). Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6078490540"
resolution: null
duplicate_of: null
---
Low. Review C https://github.com/jlevy/urollup/pull/16#pullrequestreview-5468251421. codex_rollout.rs:1257-1258 requires boundary > 0; design docs/urollup-design.md:866-869 says the check applies in a child with an explicit boundary. Probe N3: counter-only child, boundary 0, no copied prefix, first counter 108/12 last 18/2, parent 100 present: counted [100, 120] = 220, Complete, no diagnostic (inherited 100 counted as the child's).
