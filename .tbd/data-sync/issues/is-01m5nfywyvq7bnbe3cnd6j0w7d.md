---
type: is
id: is-01m5nfywyvq7bnbe3cnd6j0w7d
title: Treat empty request and response IDs as absent
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:28.666Z
updated_at: 2026-10-23T23:01:28.666Z
---
An empty Claude requestId or message.id, or an empty Codex response_id, fails key construction (scope.rs:413 rejects empty NativeId text; observe and RESPONSE_KEY.key propagate with ?), so one such line anywhere, even inside another thread's copied prefix, makes every whole-history command exit 1. Fix: read an empty ID as absent, like null, with a diagnostic.
