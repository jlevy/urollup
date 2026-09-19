---
type: is
id: is-01m2pkgv1mh7268dh4sxbptdmg
title: "Scalable ingestion phase 2: fast decode and scale gates"
kind: task
status: open
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
dependencies:
  - type: blocks
    target: is-01m2pkgva22qd452me3cdjc1fq
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
child_order_hints:
  - is-01m2x8apr4dv9jctc0j20g6mbq
  - is-01m2x8aq4skat5vmx2y97th32e
created_at: 2026-09-17T02:35:51.219Z
updated_at: 2026-09-19T16:35:16.389Z
---
Finish Phase 2 of the scalable-ingestion plan: whole-history wall time at most 10 s, with leftover Value helpers off the decode path. The 512 MiB typed-decode work that used to live here is Phase 1 (uro-q1ik Claude parse_record, uro-g7pi Codex rate_limits). The streaming synthetic corpus generator and CI scale gates already landed.

Children: uro-lsaz (10 s), uro-6gwt (leftover Value cleanup). Blocked on uro-n1cp.

Acceptance: outputs byte-identical to Phase 1; whole history at most 10 s; scale gates green in CI.

## Notes

2026-09-19: Typed 512-path decode moved to uro-n1cp children so Phase 2 is no longer blocked on the work that closes Phase 1. Remaining here: 10 s (uro-lsaz) and leftover Value helpers (uro-6gwt).
