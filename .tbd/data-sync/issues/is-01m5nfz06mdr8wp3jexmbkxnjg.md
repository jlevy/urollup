---
type: is
id: is-01m5nfz06mdr8wp3jexmbkxnjg
title: Honor CARGO_TARGET_DIR in parity targets and keep gate proofs out of the repository target
kind: chore
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - tooling
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:31.987Z
updated_at: 2026-10-23T23:01:31.987Z
---
The parity, e2e-local and parity-local Makefile recipes hard-code target/debug/urollup while the golden scripts honor CARGO_TARGET_DIR, so make check fails at parity when the target directory is elsewhere; make gate-proofs writes about 1.4 GB into the repository's target/ regardless, which matters on the maintainer's nearly full data disk. Fix: derive binary paths from CARGO_TARGET_DIR and put gate-proof scratch under it.
