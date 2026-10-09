---
type: is
id: is-01m4fdqhrppmjeyhjv4kw8996p
title: A source deleted or compressed after discovery aborts the whole report
kind: bug
status: in_progress
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T04:11:22.262Z
updated_at: 2026-10-09T04:11:47.072Z
---
read_source returned SourceReadError::Open when a discovered file no longer existed, and both adapters turn that into AdapterError::Read, so one missing file fails report, daily and sessions for the whole history. It happens in normal use: Claude Code deletes transcripts older than cleanupPeriodDays when a session starts, and zstd --rm or gzip replaces foo.jsonl with its compressed form while a report runs. Reproduced on a real history when Claude Code's startup cleanup ran during a daily run. Fix: open the next representation of the same logical source, including a sibling written after discovery; when none exists, record an empty snapshot with SourceChange::Vanished instead of failing. The CLI catalog and classification readers treat a missing file as having no header.
