---
type: is
id: is-01m2y4yge5wcpbvpnp7ye70ht1
title: Preserve the full u64 block-index domain through sequence compaction
kind: bug
status: in_progress
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-20T00:55:05.922Z
updated_at: 2026-09-20T04:24:07.574Z
started_at: 2026-09-20T01:36:18.372Z
---
Audit R3, PR #11. NativeSequence::new(u64::MAX) returns None; the Claude adapter silently converts a valid apiBlockIndex into missing sequence. Two revisions with equal output=10, block indices [u64::MAX,1] and input counts [100,1] now select 1 input token instead of 100. Preserve lossless ordering over every accepted u64 value, or explicitly validate a narrower dialect domain rather than treating overflow as absent. Test u64::MAX, MAX-1, zero, missing, reversed input order and selected totals.
