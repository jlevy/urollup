---
type: is
id: is-01m2ks74tkw1s25dhkb5q4e2q8
title: Add the local-only aggregate mode for end-to-end result checks
kind: task
status: closed
priority: 2
version: 9
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - phase-1
  - milestone-0.1
  - golden
  - privacy
dependencies:
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
  - type: blocks
    target: is-01m2nw89ncs6egan0784hkk356
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-09-16T00:17:41.714Z
updated_at: 2026-09-16T20:25:36.387Z
started_at: 2026-09-16T19:49:31.373Z
closed_at: 2026-09-16T20:20:52.292Z
close_reason: Implemented consent-gated make e2e-local and an allowlisted local aggregate writer over complete days, with path-free failures and privacy sentinel tests. The actual consented local run remains tracked by uro-d36a.
resolution: null
duplicate_of: null
---
Milestone 0.1 local-only aggregate mode for end-to-end result checks over a consented corpus with no expected.json.\n\nAcceptance:\n- Run current uncached JSON commands over the real default roots, ending the interval at the start of the current local day so active sessions cannot move totals. Do not pass --no-capture in milestone 0.1 because the engine is uncached by construction; add it to future invocations when capture exists.\n- Print only reconciled aggregate token totals per category and day, prevented naive-sum overcount, session, request and diagnostic counts, and coverage gaps. Never print a path, project name, prompt, custom model name or session, request or thread ID.\n- Share the local ccusage diff privacy sentinel, run only by maintainer consent, never in CI, and document the make target beside parity-local.

## Notes

Implemented in stacked PR #8 on branch codex/v0.1-terminal-ux-acceptance. The consent-gated tooling is complete; the actual maintainer-log run remains tracked by uro-d36a. Milestone 0.1 is uncached, so a future --no-capture flag is not a prerequisite.
