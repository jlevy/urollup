---
type: is
id: is-01m2kt3aw32tw7x1p9q5f893yg
title: Check a consented corpus against a provider usage export
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
  - parity
dependencies: []
parent_id: is-01m2kt3407fwqsknjr8nvra2sy
created_at: 2026-09-16T00:33:05.401Z
updated_at: 2026-09-16T03:02:09.374Z
closed_at: 2026-09-16T03:02:09.374Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-8tcn is the original.
resolution: duplicate
duplicate_of: is-01m2ksz8r23b3kmymq8g06c90a
---
Pre-1.0 ground truth, required by the plan's Testing Strategy before 1.0. Design §4.1 and §4.2.

Acceptance:
- One consented corpus is checked against a provider usage export (Anthropic Console or OpenAI usage), at least for totals.
- Every difference is explained from source records, including usage that never reaches local logs (Codex --ephemeral threads, parallel guardian reviews, legacy remote compaction), which is reported as an unobserved coverage gap rather than zero.
- Only aggregates are recorded: no content, paths, project names, or session, request or thread IDs, matching the local parity diff's privacy rules.
- The result is recorded beside the parity records under bench/results/parity/.
