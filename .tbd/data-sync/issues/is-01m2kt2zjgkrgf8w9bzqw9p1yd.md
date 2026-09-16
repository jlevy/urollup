---
type: is
id: is-01m2kt2zjgkrgf8w9bzqw9p1yd
title: Add Pi --current detection and ccusage pi parity cases
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
  - parity
dependencies: []
parent_id: is-01m2kt2tyeqvt9kyp7fp7f22zk
created_at: 2026-09-16T00:32:53.837Z
updated_at: 2026-09-16T03:02:06.735Z
closed_at: 2026-09-16T03:02:06.730Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-kbi1 is the original.
resolution: duplicate
duplicate_of: is-01m2ksyjfgmm1by6b9pv6r2vnv
---
Phase 2: complete Pi support once the adapters are validated. Design §6.2 and §2.1; plan, ccusage reconciliation harness (Phase 2 row).

Acceptance:
- --current resolves PI_SESSION_FILE; PI_SESSION_ID without PI_SESSION_FILE marks an unsaved session and exits 1 rather than searching roots; --agent limits which variables count; Phase 1's unsupported-dialect exit 2 for a detected Pi session is removed.
- A Pi current-session summary includes the calling request, because Pi writes it before running a tool.
- Nested-agent rules still apply: an agent started inside another exposes both agents' variables and exits 2 without --agent, naming each agent, variable and choosing flag.
- `ccusage pi daily` and `ccusage pi session` cases join the reconciliation harness with --pi-path pointing at the fixture copy, and the Pi fork replay ledger entry is seeded (fixed on ccusage main in 809eeb6, not in 20.0.20).
