---
type: is
id: is-01m3gvh6vhs5keehwyxd6r88vs
title: Enforce a conservative process-wide memory admission budget
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - memory
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-27T07:16:07.152Z
updated_at: 2026-09-27T07:16:39.351Z
---
Required for 0.1 under the approved scalability policy. Replace per-agent row-shell estimates with one invocation-wide conservative reservation model covering both retained ledgers, observations and pending rows, request/reconciliation construction, variable payloads, intern tables, discovery metadata and worker buffers, with explicit headroom for allocator/measurement overhead. Preserve max-rows as a separate observation ceiling and define migration of max-ram semantics honestly. Respect container/process allowances where discoverable; document fallback and overrides. Refuse before allocating beyond the budget with an actionable diagnostic and no partial successful report. Test combined-agent, high-cardinality, dense-record and low-budget cases; preserve exact output and worker determinism. Measured footprint must stay within the validated envelope. A sampled watchdog is a validation backstop, not the runtime budget.
