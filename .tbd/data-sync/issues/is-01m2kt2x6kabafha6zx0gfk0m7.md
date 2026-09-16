---
type: is
id: is-01m2kt2x6kabafha6zx0gfk0m7
title: Build the web UI screens, badges and browser tests
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2kt2tyeqvt9kyp7fp7f22zk
created_at: 2026-09-16T00:32:51.409Z
updated_at: 2026-09-16T03:02:05.531Z
closed_at: 2026-09-16T03:02:05.530Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-h1t1 is the original.
resolution: duplicate
duplicate_of: is-01m2ksyczqzy351xyfv1c2qx8r
---
Phase 2: the embedded read-only UI. Design §7.2 and §7.1.

Acceptance:
- Frontend source lives in web/ and its built bundle is committed under crates/urollup/assets/web/ with a drift check, embedded only by the serve feature, so users install one binary and installing the crate needs no Node, uv or Python.
- Screens: scope and coverage, calendar rollups, project, account, model and effort comparison, session and descendant breakdown, request distribution and largest calls, tool totals, and request detail linked to source evidence, with drill-down preserving filters.
- Badges beside affected numbers mark unknown prices, partial coverage, ambiguous ownership, unresolved and possible usage, and non-additive groups, linked to metric definitions.
- Rendering and pagination are bounded, transcripts load only on explicit evidence requests, and the frontend renders server-calculated results rather than computing authoritative totals; only server-computed ratios are shown.
- Browser tests exercise filters, exports and badges; CLI, HTTP and the UI agree on report data for one query and snapshot.
