---
type: is
id: is-01m5nfxyjt49g8xqk2czcpqn2w
title: Keep Claude responses with distinct message IDs apart when they share a requestId
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - correctness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:57.561Z
updated_at: 2026-10-23T23:00:57.561Z
---
claude_project.rs:1551-1560 adds a provider-scoped request key to every observation, copies included, and the block selector (1771-1820) keeps one whole revision; only the model splits a group. Two main-session records with different message.id and the same requestId become 1 request (synthetic: 1,100 tokens instead of 3,150) with only claude-block-usage-conflict and complete coverage; a /btw side-question replay given the answer's requestId drops a request the same way. Fix: make message.id a revision invariant so differing message IDs never merge, and do not let a copy's keys join originals. Check real /btw records for shared requestIds when verifying.
