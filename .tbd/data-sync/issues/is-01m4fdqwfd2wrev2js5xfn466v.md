---
type: is
id: is-01m4fdqwfd2wrev2js5xfn466v
title: Codex counter decrease counts the whole cumulative total as one request
kind: bug
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies:
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T04:11:33.227Z
updated_at: 2026-10-09T04:11:48.001Z
---
On the Codex token_count counter path, RunningTotal::observe treats any category decrease as a reset and returns the new cumulative total as the delta, and counter_observation uses that delta as the request's usage. Recent Codex rollouts lower total_token_usage at compaction (the decreasing token_count sits next to a compacted record carrying latest_token_usage_record) without restarting it from zero, while the same record carries last_token_usage with the request's real usage. The request is then charged the entire running total, which can be hundreds of times larger than any context window. Synthetic reproduction: totals 100k, 200k, 300k, then a token_count with total 250k and last 10k; urollup counts 250k for that request instead of 10k. Proposed fix: on a decrease, when last_token_usage is present use it as the delta, keep the codex-counter-epoch-reset diagnostic and the new epoch baseline; keep the whole-total rule only for counters without per-request usage. Add a fixture beside counter-reset-epoch and re-run private full-history parity. A local scan found decreases in many real rollouts, every one carrying last_token_usage; values stay local.
