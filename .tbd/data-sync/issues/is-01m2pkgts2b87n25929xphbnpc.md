---
type: is
id: is-01m2pkgts2b87n25929xphbnpc
title: "Scalable ingestion phase 1: whole history works"
kind: task
status: in_progress
priority: 0
version: 58
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies: []
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
child_order_hints:
  - is-01m2wbbbwc2m0rmc4vzdqtkb67
  - is-01m2wbbd2eaetkn9zdz86xdhys
  - is-01m2wbbdhxmcebyf47j3eedj7d
  - is-01m2x8an7mrxpsqaea7f9gvdz1
  - is-01m2x8anm7hden3ve0c3274bpz
  - is-01m2wbbcf9be2tbvyz9f7yydnx
  - is-01m2x8ap0h71f5mpry8b57wb3k
  - is-01m2x8apcp21hd5yacp01m0a9b
  - is-01m2wbbfnm8gs10hrmyrg14tma
  - is-01m2xbpewbkwxm6ac5mx8pewzn
  - is-01m2xcfaq8tx365vxpxfqshk18
  - is-01m2xcy04ecenmpeaahjma25sa
  - is-01m2xdfybf0rn6n0b97jhdm8kt
  - is-01m2xdzsks8j83q8mhb1mnma02
  - is-01m2xe6nrna10q522hmn4yetgb
  - is-01m2xfem78f0v4n4fty5n86jdy
  - is-01m2xfq8c4vjc8m9s9j1vbfmcm
  - is-01m2xg2vek9txpj40mvab5rp12
  - is-01m2xgebdjvv1ybfa5qsmnxh30
  - is-01m2xgr9vmqwwf9teqq1v0bqnk
  - is-01m2xh33f1j10y2c3ykq2x8pkj
  - is-01m2xhmbbke03tcmr6f115a3jj
  - is-01m2xjc7sfjtv1wzrrv7vb4xaw
  - is-01m2xkhjsnysfmzrqtvxd2ccpk
  - is-01m2xm3qa0jxax4tsdx8m93kgc
  - is-01m2xny7rtfamywg0z6b1qxbpq
created_at: 2026-09-17T02:35:50.945Z
updated_at: 2026-09-27T07:16:40.004Z
---
Compact streaming ingestion is merged, with bounded workers, exact selectors, compact observation/request rows, interned IDs/limits, worker line-buffer bounds and typed decoding. The maintainer retired the historical 512 MiB/25-second Phase 1 acceptance targets on 2026-09-27; do not add field-shrink tasks merely to reach them or retry the documented reverted experiments. Keep this phase open pending the revised Phase 2 safety and scale gate uro-zrr0 and recorded whole-history/worker-parity acceptance. Historical notes below describe prior targets and measurements, not current requirements.

## Notes

2026-09-19: Phase 1 row compaction exhausted. uro-l0gd closed as remasure (653/586 in the spec). This bead now depends on uro-zrr0: the 512 gate lives on Phase 2 typed decode / allocator. Do not add more Phase 1 shell-field children. Not closable until zrr0 meets 512.
