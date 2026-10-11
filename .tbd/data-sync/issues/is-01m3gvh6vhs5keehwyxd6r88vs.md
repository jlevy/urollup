---
type: is
id: is-01m3gvh6vhs5keehwyxd6r88vs
title: Enforce a conservative process-wide memory admission budget
kind: task
status: open
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - memory
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
child_order_hints:
  - is-01m4p720gxeqghmsm4dyz3zj03
created_at: 2026-09-27T07:16:07.152Z
updated_at: 2026-10-11T19:29:26.036Z
---
Required for 0.1 under the approved scalability policy. Replace per-agent row-shell estimates with one invocation-wide conservative reservation model covering both retained ledgers, observations and pending rows, request/reconciliation construction, variable payloads, intern tables, discovery metadata and worker buffers, with explicit headroom for allocator/measurement overhead. Preserve max-rows as a separate observation ceiling and define migration of max-ram semantics honestly. Respect container/process allowances where discoverable; document fallback and overrides. Refuse before allocating beyond the budget with an actionable diagnostic and no partial successful report. Test combined-agent, high-cardinality, dense-record and low-budget cases; preserve exact output and worker determinism. Measured footprint must stay within the validated envelope. A sampled watchdog is a validation backstop, not the runtime budget.

## Notes

2026-10-11: stack #30 fully merged. #27 design (77569e0), #28 slices 1-2 budget source and ledger (57cdc3d), #29 slice 3 cost model and model-bound harness (df8c5ac). Remaining: slices 4-8 (readers and workers, Codex wiring, Claude wiring, CLI semantics flip, calibration), then uro-z1h1 and uro-erqo. PR #19 (pricing context) must update kappa pins after #29.
