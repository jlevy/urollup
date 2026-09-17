---
type: is
id: is-01m2pkgv1mh7268dh4sxbptdmg
title: "Scalable ingestion phase 2: fast decode and scale gates"
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
dependencies:
  - type: blocks
    target: is-01m2pkgva22qd452me3cdjc1fq
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
created_at: 2026-09-17T02:35:51.219Z
updated_at: 2026-09-17T02:36:08.745Z
---
Borrowed typed decode with memmem prefilters in both adapters, a streaming synthetic corpus generator, and CI scale gates: raw-bytes independence, footprint extrapolation and a 512 MiB watchdog run in make test. Acceptance: byte-identical output to Phase 1, whole history in at most 10 s, scale gates green in CI.
