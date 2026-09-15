---
type: is
id: is-01m2k1ytbfsevtpef58bkcar60
title: "PR #3 review R2: status label definition contradicts its use"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:11.598Z
updated_at: 2026-09-15T17:38:26.171Z
closed_at: 2026-09-15T17:38:26.168Z
close_reason: "R2 fixed in 75228c5: status label definition matches use, Open named"
resolution: null
duplicate_of: null
---
PR #3 finding R2 (Medium). docs/urollup-design.md:13-17 defines Confirmed as decided with a decision recorded in 9.1, but 20 Confirmed status lines cite no decision (e.g. :377, :679, :711, :887, :1352, :1799, :2104, :2125), and 10.3 uses a fourth label Open (:2920). Fix: redefine Confirmed as settled design not awaiting a section 10 decision, with maintainer decisions cited where they apply; list Open for 10.3.
