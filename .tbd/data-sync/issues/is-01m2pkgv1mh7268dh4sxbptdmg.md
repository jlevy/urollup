---
type: is
id: is-01m2pkgv1mh7268dh4sxbptdmg
title: "Scalable ingestion phase 2: fast decode and scale gates"
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
dependencies:
  - type: blocks
    target: is-01m2pkgva22qd452me3cdjc1fq
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
created_at: 2026-09-17T02:35:51.219Z
updated_at: 2026-09-19T07:40:13.650Z
---
Finish Phase 2 of the scalable-ingestion plan: borrowed typed decode with memmem prefilters on the remaining hot path, and whole-history wall time at most 10 s. The streaming synthetic corpus generator and CI scale gates (raw-bytes independence, footprint extrapolation, 512 MiB watchdog in make test) already landed. Acceptance: outputs byte-identical to Phase 1; whole history at most 10 s; scale gates green in CI.

## Notes

2026-09-19: Generator and CI scale gates landed early on the local scalable-ingestion branch. Remaining: typed decode on the leftover hot path and the 10 s target. Blocked on uro-n1cp (512 MiB Phase 1).
