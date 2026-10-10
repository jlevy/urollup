---
type: is
id: is-01m2ke36qgfvdvnhw7c7v5esm6
title: "Milestone 0.1: Uncached Claude Code and Codex reports"
kind: epic
status: open
priority: 1
version: 79
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2ke45tasy262pas37jxwss5
  - type: blocks
    target: is-01m2ksyxh3c3qvgg8qhp5fgbsx
  - type: blocks
    target: is-01m2ksyz8vzvrj9r0tj246h08q
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
child_order_hints:
  - is-01m2p3evvpmcn8cf59wc1196f6
  - is-01m2ksd5yzhp73gb475pzbvvg6
  - is-01m2nrq5y06fsr5zjk075ks6fc
  - is-01m2ksns5eqx65np449amxw7wd
  - is-01m2ksp7t02wzxrgvs75js0a7d
  - is-01m2ks757afpt4nq41pktqk07a
  - is-01m2ksma5tmkfs0acfc62p7z23
  - is-01m2ksmr7x9e0t324y8zfx071j
  - is-01m2ksms2rd2kwm66zpy8p159d
  - is-01m2ksn7rykwftnqzpq1xgmys3
  - is-01m2yrfmke8j4c9je5v03s67bq
  - is-01m2yrfn1gy2xtyxayev3rgr7t
  - is-01m3jd1gd0n3w6ck4kj09kat05
  - is-01m3jx84rbxf06bdgrx0q9kn74
  - is-01m4fahrdcbg0m87y2mr5vvb30
  - is-01m4fdqhrppmjeyhjv4kw8996p
  - is-01m4fdqj9jcaznh6n663rb94b2
  - is-01m4fdqwfd2wrev2js5xfn466v
  - is-01m4fdqwy15hznwazg6cjweps6
  - is-01m4fdr8epn84nfkrtbxsd1txr
  - is-01m4fjgjt4ddq8sc99h0g6n2rm
  - is-01m4fjgkg90srrjzxn7ks96dvr
  - is-01m4fsf30nh05hp8x1w0qd3ktf
  - is-01m4fsqbwsmq8wy969gz7b560e
  - is-01m4fwh3f769xec24nepn69azv
  - is-01m4fzvz3amsqnkzs9mkj0z9ag
  - is-01m4fzw02ksw7ym4ddgdbt5trk
  - is-01m4h6ajb1tdgxgnyj01fqckyn
  - is-01m4h6ak4v64tkh41cqv8zewat
  - is-01m4h6akfq2kdrfd75e1qk2gaj
  - is-01m4hp26p4y86xcegwy4qx391n
  - is-01m4hpsv6g8dcszt5kxjpzjhw5
  - is-01m4hsgeeskftt5efm505njecp
  - is-01m4kcwscv8bh3a25smqhjkxc9
  - is-01m4kj8q7ecjrbfpdaecw6ymyd
  - is-01m4kj8rvykmrj81xmf58pfkpe
  - is-01m4kkr3gvn1gn2pfef0t5xves
created_at: 2026-09-15T21:03:18.178Z
updated_at: 2026-10-10T19:13:29.626Z
---
The uncached Claude Code and Codex reporting engine is merged at main 4c55617, with completed technical review and green automated checks. Current real-history QA found a confirmed Codex paginated-fork double count (uro-kpbp): an entirely synthetic parent=100, child-own=20 case reports 220 instead of 120, with complete coverage and no diagnostics. Do not treat whole-history Codex totals as trustworthy or accept the public alpha until fixed and revalidated. Claude exploratory use has stronger independent-comparison evidence, with private results retained locally. Other remaining work: CLI honesty uro-oz6w; aggregate QA uro-qg1a; process-wide memory admission uro-6pi8, density scale proofs uro-z1h1 and accepted-head evidence uro-erqo under uro-zrr0; G1 uro-d36a; full-history QA uro-ky6c; recorded or deferred maintainer policies and unverified provider shapes. The representative 512 MiB and 10-second gates were retired on 2026-09-27. Require manageable whole-process memory, a conservative 100 GiB projection within 25% of reference-machine RAM, raw-byte streaming within the retained-state envelope and safe early refusal for dense histories. Small synthetic regression limits remain. Spill and further decode optimization are follow-ups. Release packaging uro-30ef follows acceptance. Green CI and documentation plans do not constitute release acceptance.

## Notes

2026-09-19 audit uro-y7zm: The published implementation stack is #4 -> #8 -> #10 -> #11 -> #12. uro-zrr0 owns BOTH the 512 MiB and 10 s gates; uro-n1cp depends on it. Then uro-d36a and uro-ky6c. Independent review uro-nncx now covers the published stack. Six maintainer decision beads and uro-89s7 remain open; packaging uro-30ef waits on acceptance. Earlier notes naming completed Phase 1 children as the next cuts are obsolete.

2026-09-20 readiness refresh: independent technical review uro-nncx and all 15 implementation/planning findings are closed. PR4/8/10/11/12 have green CI; no PR has merged and there are no formal GitHub approval reviews. Milestone remains open for performance uro-zrr0, consented G1/full-history QA, maintainer policy decisions and provider-shape evidence or explicit deferral. Release packaging remains blocked on milestone acceptance.
