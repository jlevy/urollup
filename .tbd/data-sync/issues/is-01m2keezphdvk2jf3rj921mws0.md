---
type: is
id: is-01m2keezphdvk2jf3rj921mws0
title: Implement snapshot manifests and complete-record source reading
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2f0tewzgw6zd3vzrp2c865s
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-15T21:09:44.258Z
updated_at: 2026-09-15T21:09:59.566Z
---
Milestone 0.1: snapshot manifest of file identity, byte extent, fingerprint and cutoff; streaming complete-line reads with pending tails, interior corruption and mutation detection; zstd rollout decoding; lenient decoding (nested nulls, any RFC 3339 precision, counted malformed lines); symlinks only within declared roots; evidence references; per design §2.2.
