---
type: is
id: is-01m2y4zmb44cb9crzfq7ezf1p0
title: Run synthetic scale gates in GitHub CI and prove their wiring
kind: bug
status: closed
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-20T00:55:42.689Z
updated_at: 2026-09-20T05:17:03.970Z
started_at: 2026-09-20T01:36:18.463Z
closed_at: 2026-09-20T05:17:03.970Z
close_reason: Fixed in PR10 at b390982f0efb7323f46ebe34e471be15e4f3bfeb; all 15 final hosted checks passed. Early-admission regressions, measured synthetic refusal and actual Ubuntu/macOS scale execution are recorded in the governing review. Full integrated make check passed, including all 30 negative gate probes.
resolution: null
duplicate_of: null
---
Audit R4, PR #10. Makefile test includes scale-gate, but ci.yml invokes individual Rust, QA-tool, golden and result commands and never make test, make check, scale-gate or check-scale.py. QA unit tests test gate logic only; the failing-test negative probe stops before scale-gate. Add an explicit supported POSIX release scale job/step behind supply-chain, and a regression test/probe for the workflow wiring. Archive measured output. Small-corpus smoke gates still do not certify the maintainer 512 MiB / 10 s target.
