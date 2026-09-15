---
type: is
id: is-01m2gxx2qpvhv6wwvv5cp8z0v3
title: "Track metaproc issue: pooled attempts delete native agent session logs"
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
refs:
  - kind: issue
    url: https://github.com/jlevy/metaproc/issues/81
    at: 2026-09-14T21:41:56.675Z
  - kind: pr
    url: https://github.com/jlevy/metaproc/pull/82
    at: 2026-09-14T21:41:56.682Z
labels:
  - external
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
created_at: 2026-09-14T21:41:51.463Z
updated_at: 2026-09-15T00:17:27.698Z
closed_at: 2026-09-15T00:17:27.697Z
close_reason: "Fixed upstream: jlevy/metaproc#82 merged 2026-09-15 (32cde09), closing #81. Pooled Codex rollouts, and Claude transcripts when persistence is on, are now copied before slot teardown to <run>/.logs/native/<step>[/<item>]/<session-stem>.codex-sessions/ and .claude-projects/; credentials are never copied."
resolution: null
duplicate_of: null
---
metaproc deleted pooled Codex rollouts (and Claude transcripts when persistence is on) at credential-slot teardown, leaving only captured streams without timestamps, model or per-response IDs. Tracked in jlevy/metaproc#81 with fix PR jlevy/metaproc#82. urollup needs the preserved logs under <session-stem>.codex-sessions/ and .claude-projects/ beside metaproc captures; once merged, reconsider the pending capture-metadata decision (uro-gxen item 1) and teach the metaproc port (uro-i6o2) the preserved layout.
