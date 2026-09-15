---
type: is
id: is-01m2k2n055vsse8f5yt27psrsf
title: "PR #3 review R12: band dimension not self-describing for repricing diagnostic"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:43:18.436Z
updated_at: 2026-09-15T17:45:32.570Z
closed_at: 2026-09-15T17:45:32.569Z
close_reason: "R12 fixed in 017fc62: band records threshold and side; exact-vs-diagnostic rule stated"
resolution: null
duplicate_of: null
---
PR #3 round 2 finding R12 (Medium). docs/urollup-design.md:1231 and :1244-1247: the usage row records the context band that applied under the export pricing basis, but no per-extent pricing basis is kept after a merge, so a later repricing cannot tell whether thresholds differ. Fix: record the threshold in force at export and whether the row exceeded it (band: {threshold, above}; null when no band); define when repricing is exact versus a diagnostic; update field table, rule and example.
