---
type: is
id: is-01m2pkgv1mh7268dh4sxbptdmg
title: "Scalable ingestion phase 2: fast decode and scale gates"
kind: task
status: open
priority: 1
version: 14
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
dependencies:
  - type: blocks
    target: is-01m2pkgva22qd452me3cdjc1fq
  - type: blocks
    target: is-01m2pkgts2b87n25929xphbnpc
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
child_order_hints:
  - is-01m2x8apr4dv9jctc0j20g6mbq
  - is-01m2x8aq4skat5vmx2y97th32e
  - is-01m2xqdmmzccchjg7wb9x4kjh5
  - is-01m2xqdn0pvctsgvsnhna3w9cn
  - is-01m2xqdnc3rt0x1zvx49m19n4a
  - is-01m2xqdnqejxxt9dexr6218q7f
created_at: 2026-09-17T02:35:51.219Z
updated_at: 2026-09-19T21:01:57.825Z
---
Finish Phase 2 of the scalable-ingestion plan: leftover typed decode and allocator work so whole-history peak can still reach 512 MiB, then wall time at most 10 s. Phase 1 row compaction is exhausted (uro-t8ws standing 653/586). The 512 MiB gate now lives here, not on more Phase 1 children. G1 (uro-d36a) waits on this bead.

Children: uro-nuhn (Codex CompactJson numbers), uro-nzo1 (Claude number checks), uro-a3fo (sidecar / parse_record), uro-96vw (allocator), uro-lsaz (10 s), uro-6gwt (leftover Value umbrella).

Acceptance: outputs byte-identical to Phase 1; peak at or below 512 MiB; whole history at most 10 s; scale gates green in CI.

## Notes

2026-09-19: uro-nuhn reverted (quiet WH 685/21.3 vs 653/17.3). Next ready uro-nzo1 (Claude numbers) or uro-96vw (allocator). 512 still owned here.
