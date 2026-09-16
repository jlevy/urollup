---
type: is
id: is-01m2kt2wg2jfgxxa1hz8e16brv
title: Implement serve evidence reads with identity verification and byte caps
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2kt2tyeqvt9kyp7fp7f22zk
created_at: 2026-09-16T00:32:50.689Z
updated_at: 2026-09-16T03:02:05.044Z
closed_at: 2026-09-16T03:02:05.043Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-0y8x is the original.
resolution: duplicate
duplicate_of: is-01m2ksya0k0wkq02e2gfws2ndh
---
Phase 2: source evidence in the UI without exposing paths. Design §7.3 (Evidence) and §2.2.

Acceptance:
- Evidence requests name a source ID, byte offset and length inside the snapshot manifest, never a path.
- The server verifies file identity and fingerprint, returns a stale-evidence diagnostic for changed files, and caps returned bytes with an explicit truncation flag and byte count; evidence is formatted lazily.
- Tests cover unknown IDs, out-of-extent and overflowing offsets, path-like and percent-encoded IDs, changed files and the byte cap.
- The UI renders evidence as inert text, proven by a browser test; hostile log text and quoted arguments are never executed.
