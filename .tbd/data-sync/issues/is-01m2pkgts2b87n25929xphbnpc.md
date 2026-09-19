---
type: is
id: is-01m2pkgts2b87n25929xphbnpc
title: "Scalable ingestion phase 1: whole history works"
kind: task
status: in_progress
priority: 0
version: 24
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
  - type: blocks
    target: is-01m2x8apr4dv9jctc0j20g6mbq
  - type: blocks
    target: is-01m2x8aq4skat5vmx2y97th32e
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
created_at: 2026-09-17T02:35:50.945Z
updated_at: 2026-09-19T17:34:03.411Z
---
Finish Phase 1 of the scalable-ingestion plan: leftover Value decode and compact evidence refs so default whole-history sessions, daily and report --all stay at or below 512 MiB peak footprint on the maintainer corpus.

Already landed: the owner-map fix, in-place compact rows, bounded workers, guard removal, the 2 GiB compact-row ceiling, UROLLUP_STATS, the native session field, worker-count identity tests, Ingested::release_discovery (uro-brar), Claude per-record interned IDs (uro-hw2q), and worker-side Codex observe (uro-ecol). Whole history completes in about 22 s at 861 MiB.

Remaining children:
- uro-q1ik: Claude SourceDecoder::decode must not call parse_record after LineHead
- uro-g7pi: Codex RateLimitsSeed must not build Map<String, Value>
- uro-as4a: EvidenceRef 40 B → 16 B source index
- uro-l3fw, uro-1sm8: contingencies if those three miss 512 MiB
- uro-l0gd: privacy-safe RSS after each cut; closes this bead at ≤512 MiB

Acceptance: make check; whole history exits 0 in at most 25 s at no more than 512 MiB; one and eight workers give identical JSON.

## Notes

2026-09-19: uro-l3fw two-pass tried and reverted (peak 929-946 MiB). Baseline after revert is 804 MiB / 22.1 s. Remaining: uro-cbsg (inline Request.records), uro-l3fw (open, approach failed), uro-l0gd.
