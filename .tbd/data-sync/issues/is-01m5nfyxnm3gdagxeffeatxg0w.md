---
type: is
id: is-01m5nfyxnm3gdagxeffeatxg0w
title: Accept an explicit --source that holds only usage-free legacy logs
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:29.395Z
updated_at: 2026-10-23T23:01:29.395Z
---
Naming a directory with --source whose files are all pre-usage Codex rollouts (2025 JSON and early JSONL, or tiny wrapper sessions with only messages) aborts with 'contains no readable Claude Code or Codex JSONL records', although default discovery skips the same files silently. Fix: report such a source with a diagnostic and continue; keep the error for a source with no files at all.
