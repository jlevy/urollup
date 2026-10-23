---
type: is
id: is-01m5nfwrxzcd0h3c1tset1jrag
title: Decide whether the alpha ships with the row ceiling as its memory guard
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - alpha
  - design-decision
dependencies:
  - type: blocks
    target: is-01m5nfw7st3tha6eybrx2nk9z6
parent_id: is-01m5nfw7st3tha6eybrx2nk9z6
created_at: 2026-10-23T23:00:19.006Z
updated_at: 2026-10-23T23:02:41.318Z
---
The 2026-10-23 alpha readiness review found whole-history runs on the maintainer's corpus completing at a small fraction of physical RAM, while the alpha's critical path runs through admission slices 5-8 (uro-6pi8, draft PRs #40 and #41), density-scale proofs (uro-z1h1) and representative evidence (uro-erqo). Options: (a) keep them alpha gates as the frozen scope in uro-aakb says; (b) land #40/#41 when reviewed but tag the alpha with the existing row ceiling as the safety net, moving slices 7-8, uro-z1h1 and uro-erqo to the next pre-1.0 release. Either way the macOS line-buffer growth bead in Phase 1 stays an alpha fix. Record the choice in the plan and on uro-aakb.
