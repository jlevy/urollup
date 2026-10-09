---
type: is
id: is-01m4fsw5446a3a67nsnq2czqf8
title: "PR #18 B3: a Codex reset whose last_token_usage has zero input and output becomes a counted request"
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:36.067Z
updated_at: 2026-10-09T07:43:36.067Z
---
Low. Review B https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467213761. codex_rollout.rs:1101-1129; docs/urollup-design.md:848-854. Coordinator decision 5: add no request, keep the epoch diagnostic, fix design text.
