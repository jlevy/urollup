---
type: is
id: is-01m2kszpk59fx22arz90p765qs
title: Move adapters and serve into their own crates (Later)
kind: feature
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - later
dependencies: []
parent_id: is-01m2kszagjf5mxhe33418nswt5
created_at: 2026-09-16T00:31:06.333Z
updated_at: 2026-09-16T00:31:06.333Z
---
Later, not scheduled: split agent adapters and the serve feature into their own crates once a real dependency or release boundary justifies it. Design §8.1, §7.1, §10.2 and Decision 21.

Acceptance (when taken up):
- The adapter API is already library-shaped, so the move is mechanical; metaproc or another tool depending on the Rust adapters is the trigger.
- serve already lives in a self-contained module with optional dependencies and a --no-default-features CI check, so moving it means moving one module and its optional dependencies.
