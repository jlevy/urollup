---
type: is
id: is-01m4fsw25tfsxy9rx149nnq0ph
title: "PR #18 A5: unreadable_and_unnamed_sources_are_errors can pass vacuously"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:33.049Z
updated_at: 2026-10-09T07:43:33.049Z
---
Medium. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. crates/urollup-core/src/sources/reader/tests.rs:591-608. Assertion sits inside if let Err. Coordinator decision 7: skip explicitly when the process can open a mode-000 file, otherwise assert the error.
