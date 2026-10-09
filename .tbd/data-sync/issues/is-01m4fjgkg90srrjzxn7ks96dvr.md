---
type: is
id: is-01m4fjgkg90srrjzxn7ks96dvr
title: Add a fixture case for a Codex total lowered at compaction
kind: task
status: in_progress
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T05:34:57.544Z
updated_at: 2026-10-09T07:37:00.811Z
---
uro-v1c9 is covered by a synthetic adapter test. Add a codex-rollout fixture beside counter-reset-epoch whose token_count total decreases next to a compacted record while last_token_usage stays small, with expected.json, a README, its e2e golden and any ccusage parity ledger entry, so the rule is visible in the fixture corpus and the naive whole-total sum is recorded.

## Notes

2026-10-09: implemented in PR https://github.com/jlevy/urollup/pull/22 (stack #23 above #18). Close when it merges.
