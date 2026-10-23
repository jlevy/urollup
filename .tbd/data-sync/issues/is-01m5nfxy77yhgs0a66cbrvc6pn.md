---
type: is
id: is-01m5nfxy77yhgs0a66cbrvc6pn
title: Stop an unnamed copied settings event from handing a Codex parent's prefix to the child
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
created_at: 2026-10-23T23:00:57.190Z
updated_at: 2026-10-23T23:00:57.190Z
---
codex_rollout.rs:2006 (and 2821 in legacy_copied_evidence) sets active_thread = thread_id.unwrap_or(file_thread), so a copied parent thread_settings_applied without thread_id (Codex 0.144-0.151) makes the rest of the parent's copied prefix count as the child's; turn inference (1952-1959) does not run while active_thread == file_thread, so the parent's known later turn does not save it. Synthetic: parent 3,300 plus child own 550 should total 3,850; urollup reports 6,050 with complete: true and only codex-copied-history-inferred. Design lines 989-993 and 1032 say an unnamed settings event names no owner. A real-history scan found a few rollouts with both ingredients. Fix: let an unnamed settings event start the child only when no later line before the first unrecorded turn is a parent-recorded turn or total, or port squares' rule that a compaction's settings reapply is not a child start (squares commit 4ac71438).
