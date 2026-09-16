---
type: is
id: is-01m2keezphdvk2jf3rj921mws0
title: Implement snapshot manifests and complete-record source reading
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2f0tewzgw6zd3vzrp2c865s
  - type: blocks
    target: is-01m2ksn7rykwftnqzpq1xgmys3
  - type: blocks
    target: is-01m2ksnwetnwwwdp2gg33d0e3t
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-15T21:09:44.258Z
updated_at: 2026-09-16T03:03:37.009Z
closed_at: 2026-09-16T03:03:37.007Z
close_reason: "Landed in crates/urollup-core/src/sources on m01-core (merged as 69dec65): snapshot manifests (file identity, byte extent, fingerprint, cutoff, record counters, coverage failures), complete-record streaming with pending tails and counted malformed lines, zstd twin handling with decoded offsets, lenient decoding, symlinks only within declared roots, evidence references and mid-scan change detection. Windows change detection stays open as uro-jqs5."
resolution: null
duplicate_of: null
---
Milestone 0.1: snapshot manifest of file identity, byte extent, fingerprint and cutoff; streaming complete-line reads with pending tails, interior corruption and mutation detection; zstd rollout decoding; lenient decoding (nested nulls, any RFC 3339 precision, counted malformed lines); symlinks only within declared roots; evidence references; per design §2.2.
