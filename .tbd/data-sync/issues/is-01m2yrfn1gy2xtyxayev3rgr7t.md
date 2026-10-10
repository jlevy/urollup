---
type: is
id: is-01m2yrfn1gy2xtyxayev3rgr7t
title: Replace local aggregate per-session rescans and preserve metric availability
kind: bug
status: closed
priority: 1
version: 7
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-20T06:36:30.639Z
updated_at: 2026-10-10T01:19:01.867Z
closed_at: 2026-10-10T01:19:01.848Z
close_reason: "Merged in PR #24 (80bb0e2). Token availability is labeled per day row (all_days/some_days/no_days); request-level availability is uro-r67m. Plan R3 row and status lines are updated in the post-merge docs pass (uro-l9vb)."
resolution: null
duplicate_of: null
---
R3: tests/parity/local_aggregate.py::stable_session_summary starts daily once per session after three whole-history commands. METRICS omits cache-write lifetime buckets and integer maps missing/null to zero. Replace the N-session process loop with a bounded shared query or batched join; do not drop stable-session coverage to meet the bound. Preserve 5m/1h/unspecified cache writes and observed-zero versus unknown/partial fields in a versioned aggregate schema. Test process count independent of session count, stable cutoff semantics, null/zero distinctions, failures and existing privacy sentinels. Reuse the shared query engine; no second accounting implementation.

## Notes

PR #24 (https://github.com/jlevy/urollup/pull/24), branch fix/local-aggregate-shared-query, head 0059751. Review A addressed (https://github.com/jlevy/urollup/pull/24#issuecomment-6089705866): POSIX-rule zone in tests (Windows green), token_day_coverage labels by day row (request-level follow-up uro-r67m), sessions.excluded split with whole-history classification documented, version and sessions contract checked first. CI run 37994216803 green on all 15 checks. Close when the PR merges.
