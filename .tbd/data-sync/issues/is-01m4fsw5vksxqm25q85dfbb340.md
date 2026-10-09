---
type: is
id: is-01m4fsw5vksxqm25q85dfbb340
title: "PR #18 C2: a rollout that vanishes before the catalog read drops out of --session selections"
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:36.818Z
updated_at: 2026-10-09T07:43:36.818Z
---
Low. Review C https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467177624. crates/urollup/src/cli.rs:811-816, 665-706. Coordinator decision 6: keep a source whose header cannot be read in the narrowed selection, try its other files first.
