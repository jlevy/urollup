---
type: is
id: is-01m2ksntwyn71sp9tdqxrvv5wn
title: Add insta snapshots of reconciled ledgers over the frozen fixtures
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - testing
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:43.069Z
updated_at: 2026-09-16T00:25:43.069Z
---
The engineering baseline brief's testing layers put insta YAML snapshots of normalized ledgers and report data between unit tests and property tests. uro-spce and uro-26dh landed with unit and property tests but no snapshots, because insta was deferred until there was ledger data to snapshot and the fixtures in uro-obx5 did not exist yet.

Add insta (checking the crates.io publication date against the 14-day cool-off and recording it in SUPPLY-CHAIN-SECURITY.md) and snapshot the reconciled ledger and manifest for a few small fixtures, including the double-counting cases from the research brief, so a change in reconciliation shows up as a reviewable diff.
