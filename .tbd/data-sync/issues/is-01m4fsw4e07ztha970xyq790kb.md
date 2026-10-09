---
type: is
id: is-01m4fsw4e07ztha970xyq790kb
title: "PR #18 B1: an empty or partial primary accepts every twin unread, dropping usage while totals stay complete"
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:35.359Z
updated_at: 2026-10-09T07:43:35.359Z
---
High. Review B https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467213761. reader.rs:791. fingerprint None skips the comparison and lists every twin as verified. Coordinator decision 1: when the primary has no first record and a twin does, read that twin and record the skip.
