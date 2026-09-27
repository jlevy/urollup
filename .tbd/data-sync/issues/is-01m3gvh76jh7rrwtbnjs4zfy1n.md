---
type: is
id: is-01m3gvh76jh7rrwtbnjs4zfy1n
title: Prove observation-density scaling and the 100 GiB operating envelope
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-27T07:16:07.505Z
updated_at: 2026-09-27T07:16:39.362Z
---
Required for 0.1. Measure at least three increasing unique-observation counts across Claude-heavy, Codex-heavy and mixed synthetic histories, including high-cardinality limits/IDs and long session families. Separate input bytes from retained observation/request counts. Keep the existing fixed-observation payload-padding regression. Record complete invocation peaks for sessions, daily and report, and compare one/eight workers with exact output parity. Use a conservative upper linear envelope and documented margin to project 100 GiB at the measured reference density; require peak within 25% of reference-machine RAM, no superlinear trend, and normal machine pressure. Label projections as estimates, not a measured 100 GiB claim. Prove streaming of a synthetic input exceeding the enforced memory allowance using bounded generation and external scratch; do not copy private logs or create a giant corpus just for nominal size. Validate early refusal on dense over-budget inputs. Keep private measurements local.
