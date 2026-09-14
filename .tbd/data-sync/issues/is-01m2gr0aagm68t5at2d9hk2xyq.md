---
type: is
id: is-01m2gr0aagm68t5at2d9hk2xyq
title: Walk through design decisions from code reviews
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - planning
dependencies:
  - type: blocks
    target: is-01m2f0snm56a9zh7xm2zh4hw13
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
created_at: 2026-09-14T19:58:46.095Z
updated_at: 2026-09-14T19:58:47.207Z
---
Walk the maintainer through the design decisions raised by the source and code reviews one at a time, recording each confirmed decision in the plan and data contracts and creating or updating beads.

## Notes

Design decisions raised by the 2026-09-14 code reviews, in walkthrough order (most consequential first). Each needs maintainer confirmation before the plan changes.
1. Capture cache phase and retention (uro-1k0u): spike shows uncached engine is fast enough for Phase 1; decide cache phase and whether captured records outlive deleted logs (and live in a data directory rather than a purgeable cache directory).
2. Harness captured sources: add a metaproc run-directory source layout (.logs/tasks/**/*.jsonl[.gz] with .invocation.json sidecars; Phase 2 bead), and state that harnesses which discard native logs (qm) are out of scope until a tested export adapter exists (possible Phase 3 harness-ledger import).
3. Accounts: optional organization or quota-group identifier per account, apiKeySource as an observed property, principals distinct from billing accounts; windows groupable by quota group (feeds the account registry, uro-um7n).
4. Unitemized usage: record source totals (claude-stream modelUsage, Pi agent_end) that exceed reconciled requests as an unitemized measure beside totals, never inside them; --strict exits 3.
5. Branch and agent grouping: observed branch (Claude gitBranch per request, Codex session_meta.git.branch per thread), Codex agent_path, agent_role and agent_nickname as thread properties; --group-by branch and agent_path.
6. Extended time measures: agent-active seconds vs busy union and parallel overlap, overlap-safe tool intervals by category, context compaction time, model-time bounds with per-dialect availability.
7. Structural command summary: a versioned shell-command classifier (ported from squares) run before tool arguments are stubbed, so command statistics survive stripping.
8. Per-extent completeness: flag open and abandoned turns at the snapshot; --strict treats open turns as a coverage gap.
9. Inferred timestamps: for records without timestamps, infer from neighboring records, then sidecar capture time and file mtime, labeled inferred; never epoch zero.
10. Anomaly detectors: port agentfdr's loop, error-streak, token-spike and stalled-call detectors as labeled estimates in check or the reporting skill.
11. squares as first integration user: a bead to replace squares' log rollups with urollup summaries (expected drop in totals), plus its needs: softschema.schema pointer on export, flowmark-stable Markdown, PyPI wheel priority.
