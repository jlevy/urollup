---
type: is
id: is-01m2gr0aagm68t5at2d9hk2xyq
title: Walk through design decisions from code reviews
kind: task
status: open
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - planning
dependencies:
  - type: blocks
    target: is-01m2f0snm56a9zh7xm2zh4hw13
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
created_at: 2026-09-14T19:58:46.095Z
updated_at: 2026-09-15T20:15:21.540Z
---
Walk the maintainer through the design decisions raised by the source and code reviews one at a time, recording each confirmed decision in the design doc (docs/urollup-design.md §10.1 Design Decisions, with the queue in §9.2) and creating or updating beads.

## Notes

Design decisions raised by the 2026-09-14 code reviews, in walkthrough order. Each needs maintainer confirmation before the plan changes.
Resolved 2026-09-14: capture store in Phase 1 with cache read path in Phase 2 (uro-1k0u); harness logs read directly through urollup's own adapters with metaproc code ported in, no metaproc dependency, and harness log deletion fixed in metaproc; qm out of scope except as inspiration and borrowable MIT code.
Remaining:
1. Capture metadata for captured streams: whether urollup defines a small softschema capture-metadata contract (capture time, requested model and effort, cwd, account, harness) that harnesses write beside captured streams, reads metaproc's .invocation.json as-is, or uses records only plus source mappings. Narrowed 2026-09-15: metaproc#82 now preserves pooled Codex rollouts (and Claude transcripts when persistence is on), so metadata matters mainly for Claude and Pi runs with persistence off and for non-pool cloud runs.
2. Accounts: optional organization or quota-group identifier per account, apiKeySource as an observed property, principals distinct from billing accounts; windows groupable by quota group (feeds the account registry, uro-um7n).
3. Unitemized usage: record source totals (claude-stream modelUsage, Pi agent_end) that exceed reconciled requests as an unitemized measure beside totals, never inside them; --strict exits 3.
4. Branch and agent grouping: observed branch (Claude gitBranch per request, Codex session_meta.git.branch per thread), Codex agent_path, agent_role and agent_nickname as thread properties; --group-by branch and agent_path.
5. Extended time measures: agent-active seconds vs busy union and parallel overlap, overlap-safe tool intervals by category, context compaction time, model-time bounds with per-dialect availability.
6. Structural command summary: a versioned shell-command classifier (ported from squares) run before tool arguments are stubbed, so command statistics survive stripping.
7. Per-extent completeness: flag open and abandoned turns at the snapshot; --strict treats open turns as a coverage gap.
8. Inferred timestamps: for records without timestamps, infer from neighboring records, then capture metadata and file mtime, labeled inferred; never epoch zero.
9. Anomaly detectors: port agentfdr's loop, error-streak, token-spike and stalled-call detectors as labeled estimates in check or the reporting skill.
10. squares as first integration user: a bead to replace squares' log rollups with urollup summaries (expected drop in totals), plus its needs: softschema.schema pointer on export, flowmark-stable Markdown, PyPI wheel priority.
