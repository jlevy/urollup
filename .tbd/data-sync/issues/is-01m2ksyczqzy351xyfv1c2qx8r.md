---
type: is
id: is-01m2ksyczqzy351xyfv1c2qx8r
title: Build the web UI screens, badges and browser tests
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2ksdvyydvhgtpxqma9kwjkj
  - type: blocks
    target: is-01m2ksdx4757n11wv9vnhmpftm
parent_id: is-01m2ksy5yrxn48thxc311sqsv6
created_at: 2026-09-16T00:30:23.725Z
updated_at: 2026-09-16T00:32:00.333Z
---
Phase 2: the embedded read-only UI. Design §7.2 and §7.1.

Acceptance:
- Frontend source lives in web/ and its built bundle is committed under crates/urollup/assets/web/ with a drift check, embedded only by the serve feature, so users install one binary and installing the crate needs no Node, uv or Python.
- Screens: scope and coverage, calendar rollups, project, account, model and effort comparison, session and descendant breakdown, request distribution and largest calls, tool totals, and request detail linked to source evidence, with drill-down preserving filters.
- Badges beside affected numbers mark unknown prices, partial coverage, ambiguous ownership, unresolved and possible usage, and non-additive groups, linked to metric definitions.
- Rendering and pagination are bounded, transcripts load only on explicit evidence requests, and the frontend renders server-calculated results rather than computing authoritative totals; only server-computed ratios are shown.
- Browser tests exercise filters, exports and badges; CLI, HTTP and the UI agree on report data for one query and snapshot.
