---
type: is
id: is-01m2ksx6f9w2rep42kwatz76k5
title: Add golden repricing tests
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
dependencies: []
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-16T00:29:44.277Z
updated_at: 2026-09-16T00:29:44.277Z
---
Milestone 0.4: prove historical repricing is exact and stable, so a table update is reviewable. Design §4.5.

Acceptance:
- Golden scenarios cover a promotion whose end date moves, a mid-life cache-read rate cut, a 200K-token context band, a 1-hour cache-write rate, an alias-only model ID, an unpriced placeholder model, a default-assumed tier and a stacked dated modifier row.
- Money is exact decimal arithmetic end to end, with clippy::arithmetic_side_effects denied in the money modules and no f64 in any priced amount.
- A table update changes goldens only where rates changed, and the tests assert the table review date is not older than any row.
- Bundled rates change only through a reviewed table update with these tests, shipped in a release.
