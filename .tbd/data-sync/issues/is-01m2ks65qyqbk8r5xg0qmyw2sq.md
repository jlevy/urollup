---
type: is
id: is-01m2ks65qyqbk8r5xg0qmyw2sq
title: Harden the golden harness and add end-to-end result checks
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - golden
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:17:09.884Z
updated_at: 2026-09-16T00:18:38.173Z
closed_at: 2026-09-16T00:18:38.172Z
close_reason: "Harness landed on branch m01-golden: hermetic environment, corpus lint, runner guards, the results checker with its samples and tests, the fixture-to-golden mapping, make targets, CI steps and gate probes"
resolution: null
duplicate_of: null
---
Milestone 0.1: audit the tryscript golden harness against golden-testing-guidelines, general-testing-rules and the pinned tryscript 0.2.1, and build the harness so later beads only add cases. See the golden testing audit (docs/project/research/research-2026-09-15-golden-testing-audit.md), the plan's Golden and end-to-end result checks, and tests/golden/README.md.

- scripts/golden-env.mjs: allowlisted environment, hermetic HOME with XDG and capture directories, canary logs in every default discovery root, and a write guard.
- scripts/check-golden-invocations.mjs: sandbox, ambient-environment, hook, skip/only, unknown-wildcard, shell-portability, timezone and canary rules; recursive corpus discovery, shared with scripts/check-portability.mjs.
- scripts/run-golden.mjs: hermetic run, unstaged-change guard on --update, pass count equal to the selected console blocks.
- scripts/check-e2e-results.mjs with tests: fixture-case discovery, the expected.json results contract, provisional extractors, the field diff, the naive-sum overcount, determinism and the pending ratchets, over samples in tests/golden/samples/.
- scripts/new-e2e-golden.mjs for the fixture-to-golden mapping, tests/golden/e2e.config.json, tests/golden/tryscript.config.mjs patterns, make golden, make golden-update, make e2e-results wired into make test and CI, and gate probes for a skipped block, an ambient environment reference and a stale pending entry.
