---
type: is
id: is-01m2ks6m5jg3tqh3dv22mqmv17
title: Run the first real end-to-end goldens and result checks on fixtures
kind: task
status: closed
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
  - golden
dependencies:
  - type: blocks
    target: is-01m2ks74tkw1s25dhkb5q4e2q8
  - type: blocks
    target: is-01m2ks7q1qfsdbrvb62ed5g79a
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:17:24.656Z
updated_at: 2026-09-16T06:56:40.052Z
closed_at: 2026-09-16T06:56:40.051Z
close_reason: "Completed all 28 fixture transcript goldens with default-discovery and explicit --source/--no-default-sources sessions views (231 blocks total), real result extractors, deterministic JSON/table output, direct JSONL/zstd input detection, and updated gate proof. make check and PR #4 CI pass."
resolution: null
duplicate_of: null
---
Milestone 0.1: turn on the end-to-end layer once report JSON and the fixture corpus exist, per the plan's Golden and end-to-end result checks and tests/golden/README.md. The harness is in place (uro-3ht1); this bead only adds cases and fixes the provisional parts.

- Replace the provisional extractors in scripts/check-e2e-results.mjs with the real report, daily and sessions JSON paths, and update the sample outputs in tests/golden/samples/outputs/ in the same commit.
- Delete the pending entries for report, daily and sessions and pendingFixtures from tests/golden/e2e.config.json; the checker fails until they are gone, so this cannot be deferred.
- Generate one transcript golden per fixture case with scripts/new-e2e-golden.mjs, expand it with run-golden.mjs --expand, and review every line: report, daily and sessions in table and JSON.
- Confirm the rollup rows sum to the report totals, that two runs print identical bytes, and that output carries no absolute path or wall-clock value a golden would have to elide; name any genuinely volatile field as a pattern in tests/golden/tryscript.config.mjs.
- Add sessions for explicit input (--no-default-sources with --source) beside the default-discovery cases, and replace tests/golden/e2e-scaffold.tryscript.md, which exists only until the flags parse.
- Update the gate probe e2e-results-stub-declared-implemented, whose substitution goes stale with the pending entries.
