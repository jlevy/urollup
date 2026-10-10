---
type: is
id: is-01m2pkgv1mh7268dh4sxbptdmg
title: "Scalable ingestion phase 2: practical memory safety and scale acceptance"
kind: task
status: open
priority: 1
version: 28
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
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
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
  - is-01m3gvh6vhs5keehwyxd6r88vs
  - is-01m3gvh76jh7rrwtbnjs4zfy1n
  - is-01m4jjnykczqny4qw0sxzjjkw9
created_at: 2026-09-17T02:35:51.219Z
updated_at: 2026-10-10T09:35:36.027Z
---
Own the 2026-09-27 maintainer-approved release policy in the scalable-ingestion plan. The historical representative 512 MiB and 10-second thresholds are retired, not claimed as met. Require successful reference-corpus sessions/daily/report with normal memory pressure and footprint within 25% of physical RAM; a conservative density-based 100 GiB projection within that envelope; raw-byte independence and streaming beyond the memory allowance; exact accounting and worker parity; and process-wide early admission refusal before over-budget dense histories can exhaust memory. Existing small synthetic regression limits remain. Block on uro-6pi8 (process-wide budget), uro-z1h1 (scale proofs) and uro-erqo (representative evidence). G1 still depends on this gate. Hybrid spill uro-924y and profile-led speed improvements are follow-ups, not 0.1 blockers.

## Notes

2026-09-19 audit uro-y7zm: Phase 2 owns both the 512 MiB peak and 10 s whole-history gates. Standing historical evidence remains WH 653 MiB / 17.3 s and Codex 586 MiB / 13.2 s; later quiet samples 648 / 18.2 and 573 / 14.3. Reverted uro-nuhn, uro-96vw, uro-s5vb and uro-h6iw must not be retried; uro-nzo1 is on hold for lack of a new measured hypothesis. uro-lsaz and uro-a3fo are the remaining investigation paths. CI scale wiring is missing (uro-a8fk); a green existing CI run does not execute the scale workload. Resolve audit defects and remeasure current heads before claiming acceptance.

2026-09-20 readiness refresh: all R1-R11 fixes and independent implementation reviews are complete. uro-a8fk is closed: Ubuntu and macOS synthetic scale workloads execute and pass on the published stack. uro-a3fo typed Claude sidecars is implemented and closed. Earlier notes that CI wiring is missing or sidecar implementation is the next cut are historical. Next technical work is a consented exact-head representative baseline and profile under uro-lsaz; both 512 MiB and 10 s remain unmet/unproven on updated heads. Do not retry reverted cuts or held uro-nzo1 without a new measured hypothesis.
