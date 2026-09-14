---
type: is
id: is-01m2f0tej82vk57g0eaapfr88r
title: Implement analytical identities, ledger, reconciliation and ownership
kind: task
status: open
priority: 2
version: 7
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tewzgw6zd3vzrp2c865s
  - type: blocks
    target: is-01m2f0tfk4sshgqxctqqgfb2fe
  - type: blocks
    target: is-01m2f0tfxphsg75gwnpfyzb6k7
  - type: blocks
    target: is-01m2gvvvpc0hrbp287e8a08ras
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:22.152Z
updated_at: 2026-09-14T21:06:14.346Z
---
Normalized ledger, deterministic analytical identities with collision detection and re-derivation from stored keys, reconciliation, ownership status and coverage. See architecture doc: Normalized Ledger, Analytical Identities.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Property test: a group with any unknown member is partial, never complete.
- Golden cases for metaproc review defects and qm's mixed input semantics (Codex input includes cache, Claude excludes it).
- One running-total dedupe rule shared by rollouts, app-server streams and codex-exec captures, subtracting a forked child's inherited total.
- Unobserved coverage gaps for Codex --ephemeral threads, parallel guardian reviews and legacy remote compaction, never zero.
- Pi key scope: entry IDs are lineage-scoped; thr- keys use header id plus timestamp; fallback req- keys use lineage root plus a revision-invariant digest.
- Pending maintainer decisions that would add work here: an unitemized measure and inferred timestamps (see uro-gxen).
