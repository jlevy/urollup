---
type: is
id: is-01m4fsw1ew9e14r6hw1f8brsyz
title: "PR #18 A3: --session selection and explicit-source classification abort on one damaged compressed log"
kind: bug
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:32.315Z
updated_at: 2026-10-09T08:21:28.543Z
closed_at: 2026-10-09T08:21:28.542Z
close_reason: "Fixed in 8f23052 (PR #18 A3): catalog and classification pre-reads go through sources::reader::peek and never abort; a rollout with no readable header stays in narrowed selections. Tests: codex_catalog_keeps_rollouts_whose_discovered_header_cannot_be_read, a_damaged_compressed_rollout_is_reported_without_stopping_any_report."
resolution: null
duplicate_of: null
---
Medium. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. crates/urollup/src/cli.rs:808-845, 925-940 (called at 665-666 and 897-900). read_codex_session_links and classify_jsonl_with_limit turn read/decode errors into Failure::runtime and read only the primary. Coordinator decision 6.
