---
type: is
id: is-01m2kswehkpv3849n6hmp0knr6
title: Implement capture scope, --capture, --capture-idle and --no-capture
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.3
  - capture-cache
dependencies:
  - type: blocks
    target: is-01m2kswtgsj4hcyk1x890reavj
  - type: blocks
    target: is-01m2kegstn5bx58s7sc8d0s01x
parent_id: is-01m2ke4qtjw7gzyjmmzxxs64mf
created_at: 2026-09-16T00:29:19.780Z
updated_at: 2026-09-16T00:31:51.751Z
---
Milestone 0.3: which sources a run captures, and the flags that change it. Design §2.5 and Decision 9.

Acceptance:
- Capture applies only to sources found by default discovery, the user's own agent logs, including roots declared in the source manifest. Raw logs passed with --source and individual artifacts listed in the manifest are read but captured only with --capture; summaries and bundles are never captured.
- --capture-idle (5 minutes by default) captures only sources unmodified for at least that long, so `report --current` and hook-driven reports never pay capture cost for the active transcript; a later run captures it once idle. An unfinished last line stays pending and is never captured.
- --no-capture neither writes nor reads the store for a run.
- Goldens cover the idle threshold, captured manifest roots, uncaptured --source logs and manifest artifacts, and an unwritable store.
