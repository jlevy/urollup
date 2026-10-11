---
type: is
id: is-01m4fsy5m4zq4n83g3rz2ra615
title: "PR #14 A3: specify how Cursor usage enters the request ledger"
kind: bug
status: open
priority: 2
version: 6
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: null
labels:
  - post-0.1
dependencies:
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-10-09T07:44:42.115Z
updated_at: 2026-10-11T06:29:17.110Z
started_at: 2026-10-09T07:45:09.292Z
---
Severity: Medium. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Deferred from PR #14 (planning gap). Blocks the adapter bead uro-p9ay.

Where: plan-2026-09-19-cursor-dialect.md Goals usage bullet (:64-71), Usage Fields Versus Inference (:307-320), CLI and Reports (:343-346); docs/urollup-design.md §4.1 (:1097-1099) and §9.2 Unitemized Usage (:2738-2745, Candidate, queued; walk-through tracked on uro-gxen).

Problem: the most reliable usage signal, composerData usageData.costInCents, is a per-session, per-model total with no request identity and no timestamp, while the confirmed ledger records usage on requests (§3.1). Bubble tokenCount has no field-level rule.

Decide and record in the plan (a "Usage Mapping" table, one row per carrier: usageData entry, bubble tokenCount):
- Ledger entity for costInCents: synthetic request, thread-level source estimate, or unitemized measure. If it depends on the queued unitemized decision, name that dependency.
- Call count (unknown, like the Pi compaction carrier in §4.1).
- Timestamp source for daily buckets when a composer spans several days: createdAt, lastUpdatedAt, or a coverage label.
- Money measure: cents to an exact decimal amount with an explicit currency, labeled a source estimate.
- Token-field semantics: {0, 0} versus a single nonzero field.
- Cache read and write categories: unknown, never zero (§4.1 "unknown is never free"), so cache-hit share shows unknown rather than 0%.

## Notes

Deferred from PR #14 review A (tracking parent uro-l2tq). Listed in plan-2026-09-19-cursor-dialect.md under Open Questions Before Implementation at 712016e; blocks uro-p9ay. Waits on a design pass before adapter implementation; PR #14 is docs-only.
