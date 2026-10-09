---
type: is
id: is-01m4fsw428asf754gkbtv69nr8
title: "PR #18 A10: tidy duplicated copy_compressed helpers and test-only LogicalSource API"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:34.981Z
updated_at: 2026-10-09T08:21:33.083Z
closed_at: 2026-10-09T08:21:33.082Z
close_reason: "Fixed in caa9d1f (PR #18 A10) for the API half: LogicalSource::single and twins are cfg(test) pub(crate). The duplicated copy_compressed helpers stay: they are in two crates' integration tests with different twin handling, and sharing needs a test-support crate or a #[path] include."
resolution: null
duplicate_of: null
---
Suggestion (Low). Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. crates/urollup-core/tests/adapters.rs:585, crates/urollup/tests/cli_process.rs:249; reader.rs:128,163 LogicalSource::single and twins public but test-only.
