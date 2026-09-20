---
type: is
id: is-01m2y4zp4nvy4ggpg0y94dzrbe
title: Bound RAM detection and resolve PR 12 Windows golden stall
kind: bug
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-20T00:55:44.532Z
updated_at: 2026-09-20T00:58:51.423Z
---
Audit R5, PR #12 d446732. Windows run 35478104257 job 105990831189 remains in Compare the CLI golden contract since 2026-09-20 00:13:45 UTC while all other checks passed. Root cause is not yet established. capacity.rs runs powershell Get-CimInstance through unbounded Command::output; macOS runs sysctl the same way. cli.rs calls physical_memory_bytes even for explicit --max-rows or byte budgets. A synthetic three-second sysctl delayed an explicit --max-rows 1 run to 4.16 seconds. Resolve explicit byte/row settings without probing; bound/reap the host-query helper or use a reviewed native facility; preserve read-only HOME behavior; add injected probe tests and obtain green Windows goldens/results on the resulting head. Do not label the CI stall a proven PowerShell cause without logs.
