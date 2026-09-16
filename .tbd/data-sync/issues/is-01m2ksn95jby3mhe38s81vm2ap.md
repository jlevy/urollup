---
type: is
id: is-01m2ksn95jby3mhe38s81vm2ap
title: Define relationship and annotation identity keys
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.2
dependencies: []
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:25:24.911Z
updated_at: 2026-09-16T00:25:24.911Z
---
Design 3.6 gives a prefix and key for src-, thr-, req-, act- and rpt-, then says only that relationships are keyed by kind and endpoint IDs and annotations by target ID, author, method and version. Neither has a prefix, so neither has an analytical ID.

uro-spce therefore keys a Relationship by its (kind, from, to) tuple with no derived ID. That is enough in memory, but a bundle table needs a row identity and the contract needs a field for it. Decide whether relationships and annotations get their own prefixes (for example rel- and ann-) with key kinds and precedence, or whether their tuple key is the contract's identity, and write it into 3.6 and the table contracts. See crates/urollup-core/src/ledger/entities.rs.
