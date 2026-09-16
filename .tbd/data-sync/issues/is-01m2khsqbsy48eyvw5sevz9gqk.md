---
type: is
id: is-01m2khsqbsy48eyvw5sevz9gqk
title: Add local-corpus ccusage aggregate diff script
kind: task
status: closed
priority: 2
version: 8
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - phase-1
  - milestone-0.1
  - parity
dependencies:
  - type: blocks
    target: is-01m2f0th11tcymcgy8zqfqkn2g
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
  - type: blocks
    target: is-01m2nw89ncs6egan0784hkk356
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-09-15T22:08:01.912Z
updated_at: 2026-09-16T20:20:52.824Z
started_at: 2026-09-16T19:49:31.764Z
closed_at: 2026-09-16T20:20:52.823Z
close_reason: Implemented consent-gated make parity-local against pinned native ccusage with stable-cutoff daily totals, stable-session matching, safe model normalization, aggregate residuals, threshold flags, and privacy tests. The actual consented comparison remains tracked by uro-d36a.
resolution: null
duplicate_of: null
---
Milestone 0.1 maintainer-run aggregate diff between pinned ccusage and urollup over a consented local corpus.\n\nAcceptance:\n- Provide tests/parity/local_diff.py and make parity-local, run by hand or a local scheduler and never in CI.\n- Run both tools on the same default roots, timezone and stable interval ending at the start of the current local day. Do not pass --no-capture in milestone 0.1 because the engine is uncached by construction; add it to future invocations when capture exists.\n- Output aggregate tool versions, platform, timezone, interval, metric totals and deltas, matched and one-sided counts, relative-delta histogram and ledger-explained residual. Never output content, paths, project names or IDs; normalize unknown model names to other.\n- A CI sentinel fixture fails if private markers reach stdout, stderr or the output file. Record only the reviewed aggregate result for a release, and open a bead when unexplained residual exceeds the documented threshold.

## Notes

Fixture parity is implemented. This remaining local gate can run before milestone 0.3 because milestone 0.1 is already uncached; the future --no-capture flag is not a prerequisite.
