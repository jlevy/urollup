---
type: is
id: is-01m2ks74tkw1s25dhkb5q4e2q8
title: Add the local-only aggregate mode for end-to-end result checks
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - golden
  - privacy
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:17:41.714Z
updated_at: 2026-09-16T00:21:38.772Z
---
Milestone 0.1: check urollup against a consented local corpus that cannot be committed, printing aggregates only, as the plan's Golden and end-to-end result checks describes. scripts/check-e2e-results.mjs --fixtures <dir> already runs the committed checks over an uncommitted corpus; this adds the mode for a corpus with no expected.json.

- Run the JSON commands over the real default roots for a consented corpus, with the same isolation as the fixture runs and --no-capture, ending the interval at the start of the current local day so sessions still being written cannot move totals.
- Print only aggregates: reconciled totals per token category and per day, the naive-sum overcount they prevent, session, request and diagnostic counts, and coverage gaps. Never a path, project name, prompt, or session, request or thread ID.
- Follow the privacy rules of the local ccusage diff (uro-qp7j) and share its sentinel test: a fixture whose paths, project names, IDs, prompt text and custom model names carry unique markers, failing if any marker reaches stdout, stderr or the output file.
- Never runs in CI; make target beside make parity-local, and documented in tests/golden/README.md.

## Notes

Also the mechanism for checking and recording the end-to-end acceptance goals G1 and G2 (plan, End-to-End Acceptance Goals), which run on the maintainer's real local logs: the mode prints reconciled aggregates and the naive-sum overcount, and the automated goldens and result checks never read a real log.
