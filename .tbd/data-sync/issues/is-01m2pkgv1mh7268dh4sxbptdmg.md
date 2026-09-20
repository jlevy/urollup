---
type: is
id: is-01m2pkgv1mh7268dh4sxbptdmg
title: "Scalable ingestion phase 2: fast decode and scale gates"
kind: task
status: open
priority: 1
version: 22
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
  - is-01m2xr9e3p2krpf2x0vf0f09d7
  - is-01m2xsj6xgcxet8hy8dxcbeyh4
  - is-01m2y4ydxjz5xpq81vf3ecd9rv
  - is-01m2y4yf4mejx5e6b80an6f3w4
  - is-01m2y4yge5wcpbvpnp7ye70ht1
  - is-01m2y4zmb44cb9crzfq7ezf1p0
  - is-01m2y4zp4nvy4ggpg0y94dzrbe
created_at: 2026-09-17T02:35:51.219Z
updated_at: 2026-09-20T00:58:51.454Z
---
Finish Phase 2 of the scalable-ingestion plan: leftover typed decode and allocator work so whole-history peak can still reach 512 MiB, then wall time at most 10 s. Phase 1 row compaction is exhausted (uro-t8ws standing 653/586). The 512 MiB gate now lives here, not on more Phase 1 children. G1 (uro-d36a) waits on this bead.

Children: uro-nuhn (Codex CompactJson numbers), uro-nzo1 (Claude number checks), uro-a3fo (sidecar / parse_record), uro-96vw (allocator), uro-lsaz (10 s), uro-6gwt (leftover Value umbrella).

Acceptance: outputs byte-identical to Phase 1; peak at or below 512 MiB; whole history at most 10 s; scale gates green in CI.

## Notes

2026-09-19 audit uro-y7zm: Phase 2 owns both the 512 MiB peak and 10 s whole-history gates. Standing historical evidence remains WH 653 MiB / 17.3 s and Codex 586 MiB / 13.2 s; later quiet samples 648 / 18.2 and 573 / 14.3. Reverted uro-nuhn, uro-96vw, uro-s5vb and uro-h6iw must not be retried; uro-nzo1 is on hold for lack of a new measured hypothesis. uro-lsaz and uro-a3fo are the remaining investigation paths. CI scale wiring is missing (uro-a8fk); a green existing CI run does not execute the scale workload. Resolve audit defects and remeasure current heads before claiming acceptance.
