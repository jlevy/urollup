---
type: is
id: is-01m5nfyyd7qwmcqwvraw10w44v
title: Prefer the more complete of a plain log and its compressed twin
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:30.150Z
updated_at: 2026-10-23T23:01:30.150Z
---
PLAUSIBLE, not reproduced. reader.rs:182-187 and 1232-1242 verify a plain/compressed twin by its first record only and read the plain file, so a snapshot taken while a decompressor is still writing the plain file can undercount without a diagnostic. Fix: compare sizes or record counts and read the longer twin, or report the mismatch.
