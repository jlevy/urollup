---
type: is
id: is-01m4p928t0hded71eg72ereg87
title: Rerun the local real-history comparison after the 2026-10-10 accounting fixes merge
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - milestone-0.1
  - acceptance
dependencies:
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-11T20:04:31.679Z
updated_at: 2026-10-11T20:04:35.901Z
---
The 2026-10-10 local real-history rerun found uro-jqc3 (fixed in PR #32), uro-p9ua (PR #33) and uro-wvgc (PR #34); PR #19 changes Codex cache-write accounting and adds degrade diagnostics. Codex and Claude totals stay provisional until the comparison is rerun on main with all of them merged. Run whole-history report/daily/sessions against the maintainer's consented logs and compare with the previous run and pinned ccusage, aggregate only (no IDs, paths, prompts or values leave the machine). Compare diagnostic codes as well as totals: conflicting-pricing-context, codex-inclusive-input-underflow, codex-history-boundary-unverified, conflicting-owners. Include the aggregate-only checks the reviews asked for: (1) PR #34 A8/B6: among Claude subagent transcripts whose first named session differs from their directory's session, count those that are not a byte prefix of the same-named file under the named session, those with later lines naming the holding session, those whose named session has no transcript or directory anywhere, and those whose named session exists only in another project directory or root; (2) PR #33 A1: rerun the rollback-window check over migrated user forks' inferred starts (uro-eh0d); (3) confirm conflicting-owners Claude requests are gone. Record the qualitative result on uro-d36a and the release readiness record. Blocks G1 (uro-d36a).
