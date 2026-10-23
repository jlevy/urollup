---
type: is
id: is-01m5nfyztx3qcad3spte71qn8m
title: Validate --max-ram and --max-rows and say what they bound today
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - memory
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:31.612Z
updated_at: 2026-10-23T23:01:31.612Z
---
The CLI passes MemoryAdmission::unlimited().with_row_ceiling (cli.rs:401), so --max-ram is a per-agent row ceiling, not a memory cap, while its help says 'Ingest budget' (cli.rs:208). --max-rows 0 and --max-ram 8 (8 bytes) are accepted and fail at run time with exit 1 instead of as usage errors; --max-ram 1.5G is rejected; the capacity error reports maximum + 1 as the count (admission.rs:653-658) and suggests only narrower --source roots. Fix: usage-error validation, accurate help until slice 7 flips the semantics, and advice that names --max-ram. Coordinate with uro-6pi8 slice 7.
