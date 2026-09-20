---
type: is
id: is-01m2yrfn1gy2xtyxayev3rgr7t
title: Replace local aggregate per-session rescans and preserve metric availability
kind: bug
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-20T06:36:30.639Z
updated_at: 2026-09-20T06:37:56.133Z
---
R3: tests/parity/local_aggregate.py::stable_session_summary starts daily once per session after three whole-history commands. METRICS omits cache-write lifetime buckets and integer maps missing/null to zero. Replace the N-session process loop with a bounded shared query or batched join; do not drop stable-session coverage to meet the bound. Preserve 5m/1h/unspecified cache writes and observed-zero versus unknown/partial fields in a versioned aggregate schema. Test process count independent of session count, stable cutoff semantics, null/zero distinctions, failures and existing privacy sentinels. Reuse the shared query engine; no second accounting implementation.
