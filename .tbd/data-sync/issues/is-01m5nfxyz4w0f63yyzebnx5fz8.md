---
type: is
id: is-01m5nfxyz4w0f63yyzebnx5fz8
title: Report revision disagreement when Codex LatestRevision overrides differing usage
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
created_at: 2026-10-23T23:00:57.955Z
updated_at: 2026-10-23T23:00:57.955Z
---
reconcile.rs:182-221 (LatestRevision, the Codex selector at codex_rollout.rs:2772) never fills disagreements: two original revisions of one response with different usage keep the last in canonical order (src-ID hash order across files) with no diagnostic and complete coverage (synthetic: 10,700 and 59,000 token records for one response_id count 59,000 silently). The Claude selector already emits revision-disagreement. Fix: emit revision-disagreement when the selected usage differs from another revision's.
