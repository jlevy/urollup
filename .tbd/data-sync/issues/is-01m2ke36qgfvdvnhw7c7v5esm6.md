---
type: is
id: is-01m2ke36qgfvdvnhw7c7v5esm6
title: "Milestone 0.1: Uncached Claude Code and Codex reports"
kind: epic
status: open
priority: 1
version: 50
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
created_at: 2026-09-15T21:03:18.178Z
updated_at: 2026-09-20T05:35:13.371Z
---
Uncached Claude Code and Codex reports: adapters, ledger, exact selection, report / daily / sessions, goldens, ccusage fixture parity, terminal color and progress, and local acceptance tools are implemented and independently reviewed. Remaining before this epic closes: representative 512 MiB and 10 s performance (uro-zrr0), consented G1 (uro-d36a), full-history QA (uro-ky6c), and recorded or deferred maintainer decisions and unverified provider-shape evidence. Release packaging follows acceptance.

## Notes

2026-09-19 audit uro-y7zm: The published implementation stack is #4 -> #8 -> #10 -> #11 -> #12. uro-zrr0 owns BOTH the 512 MiB and 10 s gates; uro-n1cp depends on it. Then uro-d36a and uro-ky6c. Independent review uro-nncx now covers the published stack. Six maintainer decision beads and uro-89s7 remain open; packaging uro-30ef waits on acceptance. Earlier notes naming completed Phase 1 children as the next cuts are obsolete.

2026-09-20 readiness refresh: independent technical review uro-nncx and all 15 implementation/planning findings are closed. PR4/8/10/11/12 have green CI; no PR has merged and there are no formal GitHub approval reviews. Milestone remains open for performance uro-zrr0, consented G1/full-history QA, maintainer policy decisions and provider-shape evidence or explicit deferral. Release packaging remains blocked on milestone acceptance.
