---
type: is
id: is-01m2k1yw6xc2n27c3vkh00q25b
title: "PR #3 review R7: 7.3 does not state 401 for a missing or wrong token"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:13.500Z
updated_at: 2026-09-15T17:38:32.301Z
closed_at: 2026-09-15T17:38:32.300Z
close_reason: "R7 fixed in 75228c5: 7.3 states 401"
resolution: null
duplicate_of: null
---
PR #3 finding R7 (Low). Plan testing strategy :271 asserts 401; docs/urollup-design.md:1937-1945 only says every API route requires the token. Fix: add 401 to the Token bullet.
