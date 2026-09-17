---
type: is
id: is-01m2pjy02brxz362vstrafbccq
title: Claude owner maps depend on file order and drop resumed-session usage
kind: bug
status: open
priority: 0
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - correctness
dependencies:
  - type: blocks
    target: is-01m2pkgts2b87n25929xphbnpc
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-17T02:25:33.758Z
updated_at: 2026-09-17T02:36:08.265Z
---
Claude owner maps depend on discovery order, so resumed-session copies can drop or misattribute the original request.

In crates/urollup-core/src/adapters/claude_project.rs, normalize builds message_owners and uuid_owners first-wins over records in discovery order, filtering only forced_copy records. A resumed session's replayed record (sessionId differs from the file stem) is not excluded, so when the resuming session file sorts before the original, uuid_owners maps the uuid to the resuming thread. The original record then gets replay_owner set and becomes a Copy, while the replay is a Copy by the session rule, so the request is CopyOnly or ambiguous and usage is lost.

Reproduced 2026-09-16 on temporary copies of two fixtures, renaming only the resuming session file (and its own sessionId values) so it sorts first:
- gateway-message-id-reuse: 3 owned requests and 192 tokens become 1 owned plus 1 ambiguous and 140 tokens.
- brief-double-counting: 1 owned request and 40,603 tokens become 1 ambiguous and 40,015 tokens.

Fix: build owner maps in canonical order over original-eligible records only (exclude forced copies and session-mismatch replays), and add renamed-fixture cases plus a property that results are invariant under file renaming. This is the first step of the scalable ingestion redesign (uro-o6x5).
