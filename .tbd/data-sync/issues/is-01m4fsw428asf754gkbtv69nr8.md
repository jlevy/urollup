---
type: is
id: is-01m4fsw428asf754gkbtv69nr8
title: "PR #18 A10: tidy duplicated copy_compressed helpers and test-only LogicalSource API"
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:34.981Z
updated_at: 2026-10-09T07:43:34.981Z
---
Suggestion (Low). Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. crates/urollup-core/tests/adapters.rs:585, crates/urollup/tests/cli_process.rs:249; reader.rs:128,163 LogicalSource::single and twins public but test-only.
