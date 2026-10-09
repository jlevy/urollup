---
type: is
id: is-01m4fsw1ew9e14r6hw1f8brsyz
title: "PR #18 A3: --session selection and explicit-source classification abort on one damaged compressed log"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:32.315Z
updated_at: 2026-10-09T07:43:32.315Z
---
Medium. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. crates/urollup/src/cli.rs:808-845, 925-940 (called at 665-666 and 897-900). read_codex_session_links and classify_jsonl_with_limit turn read/decode errors into Failure::runtime and read only the primary. Coordinator decision 6.
