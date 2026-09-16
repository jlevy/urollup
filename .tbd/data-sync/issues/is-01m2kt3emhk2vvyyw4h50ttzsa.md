---
type: is
id: is-01m2kt3emhk2vvyyw4h50ttzsa
title: Add a ccusage blocks compatibility view labeled an estimate (Later)
kind: feature
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - later
dependencies: []
parent_id: is-01m2kt3c4nm0wzmra0pdfj59bk
created_at: 2026-09-16T00:33:09.259Z
updated_at: 2026-09-16T03:02:12.156Z
closed_at: 2026-09-16T03:02:12.152Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-9trc is the original.
resolution: duplicate
duplicate_of: is-01m2kszd3609bv92afcym95tce
---
Later, not scheduled: a ccusage-style 5-hour block view, labeled an estimate, for users migrating from ccusage. Design §4.4 and §10.2; Decision 10 keeps inferred blocks out of the accounting core and out of the parity harness.

Acceptance (when taken up):
- Blocks are inferred from activity and labeled estimates everywhere they appear, never presented as recorded usage windows.
- The view is never part of the ccusage reconciliation harness comparison (Decision 10).
