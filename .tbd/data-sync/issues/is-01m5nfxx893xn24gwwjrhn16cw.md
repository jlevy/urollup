---
type: is
id: is-01m5nfxx893xn24gwwjrhn16cw
title: Report an unended legacy Codex copied prefix as unverified, not as copies
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - correctness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:56.200Z
updated_at: 2026-10-23T23:00:56.200Z
---
codex_rollout.rs:1951-1959 with is_unknown_turn (560-564) ends a legacy copied prefix only at a turn a discovered root recorded; when nothing ends it, every own counter becomes a copy (2201-2202). A child whose parent is absent and that has no settings event, or a fork whose turn contexts lack turn_id (pre-0.100), loses all its own usage with complete: true (synthetic: a fork reporting 1,100 where 3,850 is correct, with no diagnostic when forked_from_id is absent). Broader than uro-r8si item 1. Fix: when no evidence ends a foreign prefix, report counters past the copied totals as codex-history-boundary-unverified gaps instead of copies.
