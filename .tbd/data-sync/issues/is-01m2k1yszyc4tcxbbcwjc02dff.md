---
type: is
id: is-01m2k1yszyc4tcxbbcwjc02dff
title: "PR #3 review R1: usage rows lack context band and default-assumed basis"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:11.230Z
updated_at: 2026-09-15T17:38:25.451Z
closed_at: 2026-09-15T17:38:25.449Z
close_reason: "R1 fixed in 75228c5: usage rows carry context band and default-assumed dimensions"
resolution: null
duplicate_of: null
---
PR #3 https://github.com/jlevy/urollup/pull/3, finding R1 (High). docs/urollup-design.md:1219 and :1230-1232 say usage rows carry every price-matching dimension, but :1077-1081 also matches on context band, decided per request; rows also lack whether tier and cache-write duration were observed or default-assumed, so totals.pricing.default_assumed_requests cannot be recomputed after merge. Fix: add context band and basis dimensions to usage rows and the example; state that repricing under different thresholds is a pricing-coverage diagnostic.
