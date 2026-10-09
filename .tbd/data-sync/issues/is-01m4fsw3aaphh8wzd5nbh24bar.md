---
type: is
id: is-01m4fsw3aaphh8wzd5nbh24bar
title: "PR #18 A8: simplify the Codex reset delta or move the rule into RunningTotal"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:34.217Z
updated_at: 2026-10-09T08:21:32.245Z
closed_at: 2026-10-09T08:21:32.243Z
close_reason: "Fixed in dc1389c (PR #18 A8): the reset branch passes no delta so the observation takes last_token_usage; counters.rs keeps its rule and documents the Codex override."
resolution: null
duplicate_of: null
---
Suggestion (Low). Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. codex_rollout.rs:1125-1129, counters.rs.
