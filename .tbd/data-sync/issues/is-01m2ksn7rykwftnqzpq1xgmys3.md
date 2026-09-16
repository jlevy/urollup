---
type: is
id: is-01m2ksn7rykwftnqzpq1xgmys3
title: Decide whether an idle source's unterminated final record is read
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - design-decision
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:23.476Z
updated_at: 2026-09-16T00:25:23.476Z
---
Design 2.2 says an unfinished last line is recorded as pending, and uro-26dh implements exactly that: bytes after the last newline become a PendingTail and are never parsed.

That is right for an active transcript, but a saved capture that was never terminated with a newline (a redirected claude -p stream cut off, or a codex exec capture whose process was killed) keeps its last record pending forever, so its usage is never counted and only shows up as pending bytes in the manifest. Decide whether a source with no writer (for example, one idle past --capture-idle, or one named by --source rather than discovered) may read a final unterminated line that parses as a complete record, with a diagnostic, or whether pending stays absolute. See crates/urollup-core/src/sources/reader.rs and the test an_unfinished_last_line_is_pending_not_corruption.
