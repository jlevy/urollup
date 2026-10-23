---
type: is
id: is-01m5nfxvegp2z0h9q6fn9pwrcx
title: Report dangling or unmounted Codex session roots instead of dropping them
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - coverage
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:54.341Z
updated_at: 2026-10-23T23:00:54.341Z
---
rollout_roots (codex_rollout.rs:1071-1087 on beaede2) keeps sessions/ and archived_sessions/ only when candidate.is_dir(), which is false for a dangling symlink or an unmounted volume, so the root never reaches discovery: no missing-root diagnostic, no coverage gap, complete: true, exit 0. This is live on the maintainer's machine, where archived_sessions points at an external volume that is not always mounted; the only visible trace is thread-orphan diagnostics for children whose parents were archived. Fix: pass every candidate whose symlink_metadata exists through to discovery so a dead link becomes a missing or unreadable root; report it as a diagnostic, a coverage gap and one stderr warning. Acceptance: synthetic Codex home with a dangling archived_sessions link reports the gap; an explicit CODEX_HOME root that is missing still errors.
