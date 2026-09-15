---
type: is
id: is-01m2hsnxsyhmamxynjjptasv46
title: "PR #2 review S3: Suggestion: default capture for explicit --source inputs"
kind: bug
status: closed
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:17.181Z
updated_at: 2026-09-15T16:28:33.180Z
started_at: 2026-09-15T16:13:57.841Z
closed_at: 2026-09-15T16:28:33.177Z
close_reason: "Fixed in 2b6f811 (maintainer decision): default-on capture applies only to default-discovered sources; explicit --source logs need --capture; summaries and bundles are never captured."
resolution: null
duplicate_of: null
---
Decide whether default-on capture applies to sources passed explicitly with --source (a teammate's logs, a metaproc run directory), which may surprise users by entering the personal store. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
