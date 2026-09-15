---
type: is
id: is-01m2k2n0ep883fwhrwnqykwczs
title: "PR #3 review R13: header does not define the queued qualifier"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:43:18.741Z
updated_at: 2026-09-15T17:45:32.862Z
closed_at: 2026-09-15T17:45:32.861Z
close_reason: "R13 fixed in 017fc62: header defines Candidate, queued"
resolution: null
duplicate_of: null
---
PR #3 round 2 finding R13 (Low). docs/urollup-design.md:13-21 defines Confirmed, Candidate, Later, Open; the eleven 10.2 items use Candidate, queued (:2806 onward). Fix: add a clause defining queued as proposed but not yet reflected in the design.
