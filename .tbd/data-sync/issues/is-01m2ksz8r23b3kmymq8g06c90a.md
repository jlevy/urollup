---
type: is
id: is-01m2ksz8r23b3kmymq8g06c90a
title: Check a consented corpus against a provider usage export
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
  - parity
dependencies: []
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:52.161Z
updated_at: 2026-09-16T16:55:19.696Z
---
Pre-1.0 ground truth, required by the plan's Testing Strategy before 1.0. Design §4.1 and §4.2.

Acceptance:
- One consented corpus is checked against a provider usage export (Anthropic Console or OpenAI usage), at least for totals.
- Every difference is explained from source records, including usage that never reaches local logs (Codex --ephemeral threads, parallel guardian reviews, legacy remote compaction), which is reported as an unobserved coverage gap rather than zero.
- Only aggregates are recorded: no content, paths, project names, or session, request or thread IDs, matching the local parity diff's privacy rules.
- The result is recorded beside the parity records under bench/results/parity/.
