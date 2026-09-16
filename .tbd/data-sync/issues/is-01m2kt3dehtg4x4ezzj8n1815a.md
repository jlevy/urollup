---
type: is
id: is-01m2kt3dehtg4x4ezzj8n1815a
title: Add resource collector adapters and provider charge import (Later)
kind: feature
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - later
dependencies: []
parent_id: is-01m2kt3c4nm0wzmra0pdfj59bk
created_at: 2026-09-16T00:33:08.035Z
updated_at: 2026-09-16T03:02:11.543Z
closed_at: 2026-09-16T03:02:11.541Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-qjjs is the original.
resolution: duplicate
duplicate_of: is-01m2kszbqfpaj1tyk3mcchbvbw
---
Later, not scheduled: collect resource observations and import provider charges once a tested collector or billing export exists. Design §3.1, §9.3 (Receipts and Billing Exports), §10.2; Candidate §9.1 (Resources and Charges) currently recommends deferring both.

Acceptance (when taken up):
- Provider charges stay a separate entity, never mixed with list-price estimates or source-reported cost in the Money measure.
- Resource peaks merge by maximum only within one resource scope (§5.3).
- No import path is claimed without a tested receipt or billing export format.
