---
type: is
id: is-01m2ewb56f8fw0swxckp66adtd
title: "Research: Rust CLI engineering baseline from tbd guidelines, fdu and flowmark-rs"
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - research
dependencies:
  - type: blocks
    target: is-01m2ewa6cc86g5fefabpantvmp
  - type: blocks
    target: is-01m2ewa7zw4n7ckptq387kyet3
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:36:06.732Z
updated_at: 2026-09-14T02:59:49.235Z
started_at: 2026-09-14T02:38:24.661Z
closed_at: 2026-09-14T02:59:49.234Z
close_reason: Wrote research-2026-09-13-rust-cli-engineering-baseline.md comparing tbd Rust guidelines, fdu (afbb2ee) and flowmark-rs (f1e9337); baseline adopts fdu gates/supply chain and flowmark-rs release channels plus insta, proptest, taplo, lint-inheritance check and attestations.
resolution: null
duplicate_of: null
---
Survey tbd Rust guidelines (rust-project-setup, rust-cli-rules, rust-lint-format-rules, rust-testing-rules, rust-release-rules, rust-filesystem-rules, supply-chain-hardening, cli-agent-skill-patterns) and the local repos /Users/levy/wrk/github/fdu and /Users/levy/wrk/github/flowmark-rs. Write docs/project/research/research-2026-09-13-rust-cli-engineering-baseline.md comparing their workspace layout, toolchain/MSRV, lints, formatting, error handling, CLI conventions, testing, CI, release/distribution and supply-chain practices, and recommend a baseline for urollup at least as good as both.
