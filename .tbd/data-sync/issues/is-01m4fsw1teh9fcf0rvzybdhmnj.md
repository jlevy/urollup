---
type: is
id: is-01m4fsw1teh9fcf0rvzybdhmnj
title: "PR #18 A4: unreadable-source reporting has no Codex test and no CLI-level test"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:32.685Z
updated_at: 2026-10-09T07:43:32.685Z
---
Medium. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. codex_rollout.rs:1312-1316,1391; tests/adapters.rs:667. Coordinator decision 7: Codex truncated .jsonl.zst test, losses() unit test, CLI process test.
