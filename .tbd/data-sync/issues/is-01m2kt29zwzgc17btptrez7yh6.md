---
type: is
id: is-01m2kt29zwzgc17btptrez7yh6
title: Add capture status and capture prune
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.3
  - capture-cache
dependencies: []
parent_id: is-01m2ke4qtjw7gzyjmmzxxs64mf
created_at: 2026-09-16T00:32:31.735Z
updated_at: 2026-09-16T03:02:00.151Z
closed_at: 2026-09-16T03:02:00.149Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-nidw is the original.
resolution: duplicate
duplicate_of: is-01m2kswpq22hpbr9b3cx7rbht3
---
Milestone 0.3: the capture store's user-facing commands. Design §2.5 and §6.3.

Acceptance:
- `capture status` reports entries, sizes, dialect, adapter and capture policy versions, captured extents and retained sources, with rewritten sources linked by thread so old and new entries for one thread are visible together.
- `capture prune` removes entries by age, version or retained status, under the same locking and atomic rules as writes.
- Both default to all entries, honour UROLLUP_CAPTURE_DIR, and follow the §6.5 exit codes.
- CLI goldens over a fixture store containing a retained source and a rewritten source pair.
