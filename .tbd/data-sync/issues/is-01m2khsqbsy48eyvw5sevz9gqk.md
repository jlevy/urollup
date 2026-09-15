---
type: is
id: is-01m2khsqbsy48eyvw5sevz9gqk
title: Add local-corpus ccusage aggregate diff script
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - parity
dependencies:
  - type: blocks
    target: is-01m2f0th11tcymcgy8zqfqkn2g
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-15T22:08:01.912Z
updated_at: 2026-09-15T22:08:49.064Z
---
Milestone 0.1: maintainer-run local-corpus aggregate diff between pinned ccusage and urollup, per the plan's "ccusage reconciliation harness" section (Local corpus).

- tests/parity/local_diff.py and make parity-local; run by hand or from a local scheduler, never in CI.
- Both tools on default roots, same timezone, per-agent ccusage paths (claude, codex); interval ends at the start of the current local day; urollup with --no-capture once the capture store exists.
- Output aggregates only: tool versions, platform, timezone, interval, per-metric totals per tool, per-day and per-model deltas, matched and one-sided session counts, a histogram of per-session relative deltas, and ledger-explained deltas with the unexplained residual. Never content, paths, project names, or session, request or thread IDs; model names only when they match urollup's price table or the pinned LiteLLM snapshot, else "other".
- CI privacy sentinel test: a fixture whose paths, project names, IDs, prompt text and custom model names carry unique markers; fail if any marker reaches stdout, stderr or the output file.
- Until milestone 0.5, local ledger entries give a sign and bound; request-attributed explained amounts come with the requests command (see uro-o65n).
- Commit one reviewed record per urollup release under bench/results/parity/; an unexplained residual above 0.1% of a metric's daily total (proposed threshold) gets a bead.
- Do not read or copy private log content into docs, fixtures or tests.
