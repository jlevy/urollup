---
type: is
id: is-01m2k1yvfyqyepeb1e0mcapk0z
title: "PR #3 review R5: TOC omits linked H4 sections"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:12.763Z
updated_at: 2026-09-15T17:38:29.666Z
closed_at: 2026-09-15T17:38:29.663Z
close_reason: "R5 fixed in 75228c5: TOC lists H4 sections of 2-8"
resolution: null
duplicate_of: null
---
PR #3 finding R5 (Medium). docs/urollup-design.md:23-87 TOC lists H2 and H3 only; 13 H4 sections in sections 2-8 are link targets from the plan and about twenty in-doc links but are not navigable from the TOC. Fix: add the section 2-8 H4 entries (not 9.1 decisions, which have an index table, nor section 10 items).
