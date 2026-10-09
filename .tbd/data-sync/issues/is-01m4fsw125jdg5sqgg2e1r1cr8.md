---
type: is
id: is-01m4fsw125jdg5sqgg2e1r1cr8
title: "PR #18 A2: a compressed twin still being written is reported as a different, unread source"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:31.909Z
updated_at: 2026-10-09T07:43:31.909Z
---
Medium. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. reader.rs:791 with first_record_fingerprint at 758-773. A twin with no complete first record (None) is treated as a mismatch. Coordinator decision 1 (three-way twin verification).
