---
type: is
id: is-01m4fsw125jdg5sqgg2e1r1cr8
title: "PR #18 A2: a compressed twin still being written is reported as a different, unread source"
kind: bug
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:31.909Z
updated_at: 2026-10-09T08:21:27.407Z
closed_at: 2026-10-09T08:21:27.404Z
close_reason: "Fixed in e962ed6 (PR #18 A2): a twin with no complete first record (empty, truncated, undecodable) is neither verified nor a loss. Test: a_twin_without_a_complete_first_record_is_neither_verified_nor_a_loss."
resolution: null
duplicate_of: null
---
Medium. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. reader.rs:791 with first_record_fingerprint at 758-773. A twin with no complete first record (None) is treated as a mismatch. Coordinator decision 1 (three-way twin verification).
