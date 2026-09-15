---
type: is
id: is-01m2hsntm8nhg420xkxsr6s9wy
title: "PR #2 review R8: Retained-version semantics depend on src- ID stability across rewrites"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:13.925Z
updated_at: 2026-09-15T16:28:31.943Z
started_at: 2026-09-15T16:13:57.741Z
closed_at: 2026-09-15T16:28:31.942Z
close_reason: "Fixed in 2b6f811: a rewrite changing the first record yields a new src- ID; the old entry is retained, read and deduplicated by analytical ID; capture status links them by thread; rolled-back Codex usage keeps counting."
resolution: null
duplicate_of: null
---
src- IDs include a first-record digest, so Pi or Codex rewrites that change the first record create a new src- ID and orphan the old entry. State that the old entry is retained and read, deduplicated by analytical ID, linked by thread in capture status, and that usage of rolled-back Codex turns keeps counting. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
