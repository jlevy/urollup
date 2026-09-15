---
type: is
id: is-01m2k1ytqsav9a0asb21a3zm0d
title: "PR #3 review R3: CLI-surface status lines disagree with 10.1"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:11.991Z
updated_at: 2026-09-15T17:38:27.585Z
closed_at: 2026-09-15T17:38:27.584Z
close_reason: "R3 fixed in 75228c5: status lines aligned with 10.1 CLI-surface candidate"
resolution: null
duplicate_of: null
---
PR #3 finding R3 (Medium). docs/urollup-design.md:1718-1721 says tree, weekly, windows and single --source are Candidate; :2757-2760 says tree and windows are already confirmed (Decisions 13 and 10) but :2755-2756 still lists them as additions; :1621-1623 lists --per-session as a 6.1 flag though it lives in 5.1; :1755 labels --annotation-set Later while 10.1 lists it Candidate. Fix: align the three status lines with 10.1 and split the 10.1 recommendation into confirmed and remaining. Candidate stays a candidate.
