---
type: is
id: is-01m2gcv6s6ad8d7ntkhn1cpv81
title: Review pi, agentfdr and Anthropic session-report sources for reusable parsing
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - code-review
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T16:43:44.283Z
updated_at: 2026-09-14T17:06:49.185Z
started_at: 2026-09-14T16:43:55.810Z
closed_at: 2026-09-14T17:06:49.182Z
close_reason: "Review complete (notes in scratchpad/reviews/pi-agentfdr-session-report.md, to fold into research): Pi forks keep 8-hex entry IDs, usage duplicates come from fork/export/compaction copies and JSON event stream, proposed Pi dedupe rule; agentfdr iterations and token_count bugs, portable anomaly detectors; session-report dedupe corrections, fork subagent uuid replay, deterministic-CLI skill model."
resolution: null
duplicate_of: null
---
Review read-only clones attic/pi at d981de1 (MIT; coding-agent session format, usage writes, PI_SESSION_* variables), attic/agentfdr at e0904bf (MIT; parser, subagents, cost, anomaly heuristics) and attic/claude-plugins-official at f0dce59 (Apache-2.0; session-report analyzer) for code, tests and fixtures urollup can port with attribution, and for format facts that correct or extend the research brief's dialect survey.
