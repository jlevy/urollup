---
type: is
id: is-01m2ksntwyn71sp9tdqxrvv5wn
title: Add insta snapshots of reconciled ledgers over the frozen fixtures
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - testing
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:43.069Z
updated_at: 2026-09-16T07:21:18.349Z
closed_at: 2026-09-16T07:21:18.348Z
close_reason: "Added insta 1.48.0 after crates.io cool-off/MSRV review; recorded it in SUPPLY-CHAIN-SECURITY.md; added portable YAML snapshots of reconciled ledgers and manifests for Claude block/replay double-counting, Codex repeated cumulative counters, and Codex plain/zstd twins. Local make check and PR #4 CI run 35067597039 both pass."
resolution: null
duplicate_of: null
---
The engineering baseline brief's testing layers put insta YAML snapshots of normalized ledgers and report data between unit tests and property tests. uro-spce and uro-26dh landed with unit and property tests but no snapshots, because insta was deferred until there was ledger data to snapshot and the fixtures in uro-obx5 did not exist yet.

Add insta (checking the crates.io publication date against the 14-day cool-off and recording it in SUPPLY-CHAIN-SECURITY.md) and snapshot the reconciled ledger and manifest for a few small fixtures, including the double-counting cases from the research brief, so a change in reconciliation shows up as a reviewable diff.
