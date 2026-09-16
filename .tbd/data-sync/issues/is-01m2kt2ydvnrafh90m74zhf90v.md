---
type: is
id: is-01m2kt2ydvnrafh90m74zhf90v
title: Add compare and check commands
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2kt2tyeqvt9kyp7fp7f22zk
created_at: 2026-09-16T00:32:52.665Z
updated_at: 2026-09-16T03:02:06.097Z
closed_at: 2026-09-16T03:02:06.095Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-o3rl is the original.
resolution: duplicate
duplicate_of: is-01m2ksyg21521tjne0qxx5ss4g
---
Phase 2: report diffing and threshold checks. Design §6.3, §6.4 and §6.5.

Acceptance:
- compare reports differences between two saved JSON reports named by --baseline and --candidate, from report data alone, and names both snapshots.
- check reruns a saved query named by --query and applies threshold flags such as --max-input-tokens, exiting 4 when one is exceeded.
- The first failing stage decides the exit code: request validation (2), execution (1), coverage (3, including --strict and --require-priced), then thresholds (4).
- CLI goldens for exits 0, 3 and 4.
- Anomaly detectors in check are a queued review decision (§9.2) and stay out of scope until confirmed in uro-gxen.
