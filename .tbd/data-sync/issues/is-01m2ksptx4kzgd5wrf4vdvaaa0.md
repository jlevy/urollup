---
type: is
id: is-01m2ksptx4kzgd5wrf4vdvaaa0
title: Reconcile the fixture expected.json contract with the end-to-end result checker
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - fixtures
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:26:15.843Z
updated_at: 2026-09-16T00:26:15.843Z
---
The fixtures landed on m01-fixtures with expected.json validated by scripts/check-fixtures.mjs (format urollup-fixture-expected/v1: files, decode, threads, requests, copies, totals, diagnostics, limit_observations, naive, notes), while the golden harness (uro-3ht1, branch m01-golden) landed scripts/check-e2e-results.mjs with its own expected.json results contract and naive-sum overcount check. The two were written in parallel and have not seen each other.

When both branches are on milestone-0.1: diff the two readings of expected.json, keep one documented contract (extending the fixtures README's table), make check-e2e-results.mjs and check-fixtures.mjs agree on every case, and remove whichever duplicate validation is redundant.
