---
type: is
id: is-01m2ks74tkw1s25dhkb5q4e2q8
title: Add the local-only aggregate mode for end-to-end result checks
kind: task
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - golden
  - privacy
dependencies:
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:17:41.714Z
updated_at: 2026-09-16T18:23:11.492Z
---
Milestone 0.1 local-only aggregate mode for end-to-end result checks over a consented corpus with no expected.json.\n\nAcceptance:\n- Run current uncached JSON commands over the real default roots, ending the interval at the start of the current local day so active sessions cannot move totals. Do not pass --no-capture in milestone 0.1 because the engine is uncached by construction; add it to future invocations when capture exists.\n- Print only reconciled aggregate token totals per category and day, prevented naive-sum overcount, session, request and diagnostic counts, and coverage gaps. Never print a path, project name, prompt, custom model name or session, request or thread ID.\n- Share the local ccusage diff privacy sentinel, run only by maintainer consent, never in CI, and document the make target beside parity-local.

## Notes

Fixture result checks are implemented. This remaining local-only gate can run before milestone 0.3 because milestone 0.1 is already uncached; the future --no-capture flag is not a prerequisite.
