---
type: is
id: is-01m2gvvvpc0hrbp287e8a08ras
title: Port metaproc log-processing code into urollup adapters
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T21:06:14.346Z
updated_at: 2026-09-15T00:17:28.284Z
---
Port metaproc's Python log processing into urollup's Rust adapters with provenance (repo and commit per ported unit): dialect detection from record types, claude-stream and codex-exec captured-stream parsing (pi-events with the Phase 2 Pi adapters), gzip and harness log-rewrite awareness (leading rewrite header, dropped events, synthetic message_final), tool-name taxonomy and tool intervals, and its bug-derived test cases. Keep the adapter API clean and library-shaped so metaproc can later depend on these Rust implementations.

## Notes

- metaproc >= 32cde09 (jlevy/metaproc#82) preserves native logs beside captures: captured streams under <run>/.logs/tasks/<step>/<item>/<stem>.jsonl[.gz], and native logs under <run>/.logs/native/<step>[/<item>]/<stem>.codex-sessions/YYYY/MM/DD/rollout-*.jsonl[.zst] and <stem>.claude-projects/<project>/**/*.jsonl and *.meta.json. Discover both by content under --source <run>; a preserved rollout owns usage and its codex-exec capture (same thread.started.thread_id) is a reconciliation check. Older metaproc runs have captures only.
