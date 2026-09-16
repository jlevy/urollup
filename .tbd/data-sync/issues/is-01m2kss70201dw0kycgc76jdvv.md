---
type: is
id: is-01m2kss70201dw0kycgc76jdvv
title: Reconcile Gemini CLI cross-file session copies and duplicated buckets
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-16T00:27:33.761Z
updated_at: 2026-09-16T00:27:33.761Z
---
Gemini CLI duplicates whole session files by itself, which the discovery index, the snapshot layer and reconciliation all have to agree on. Design 2.1 and 3.4 state the rules; this bead implements the cross-source part, separately from the in-file revision rules in uro-zogi.

Three writer-side duplications:

- Bucket migration: startup copies a legacy tmp/<sha256 of project root> bucket into tmp/<slug> instead of moving it, so both can hold the same sessions, with the same sessionId, message ids and timestamps, under two roots.
- Legacy format migration: resuming a .json session appends every record to a new .jsonl file beside it; the two are one logical source whose representation changed, like a Codex rollout and its .zst twin.
- --session-file import: replays user and gemini records with their original ids, timestamps and tokens into a new session with a new id, so the copies are lineage, not new usage.

Work:

- Key a Gemini source so the same session under two buckets is recognized, and decide whether both roots are reported in sources with one marked a copy.
- Implement the ownership rule (the session that recorded a message id first in time owns it) deterministically, independent of traversal order, with a diagnostic naming the other file, and a fork or lineage edge for the import case.
- Make the discovery index treat a subagent directory's files as belonging to the parent session's tree even when the two buckets disagree.
- Cover each case with fixtures (uro-6mbl) and a parity ledger entry, since ccusage counts all three twice (uro-s6pl).
