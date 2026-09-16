---
type: is
id: is-01m2ksxzagt7azxwpe2z0cn986
title: Add bench-pr and bench-scheduled CI jobs with the regression policy
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:30:09.743Z
updated_at: 2026-09-16T00:30:09.743Z
---
Milestone 0.5: keep performance from drifting. Plan performance targets; design §8.3.

Acceptance:
- bench-pr builds each pull request and its merge base on one ubuntu-24.04 runner and alternates their runs on bench-small; bench-scheduled compares main with the latest release tag on bench-1g.
- A pull request that raises median wall time or peak RSS on bench-small by more than 10% merges only with a fix or an accepted justification; a scheduled bench-1g regression over 10% opens a bead resolved before the next release.
- A release missing a reference-laptop gate ships only with the target revised from recorded results.
- CI uploads run records as job artifacts; only reference-laptop records are committed.
- Request exports and exact percentiles declare separate memory costs in their records.
