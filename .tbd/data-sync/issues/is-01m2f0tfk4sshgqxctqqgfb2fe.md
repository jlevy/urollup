---
type: is
id: is-01m2f0tfk4sshgqxctqqgfb2fe
title: Author summary and bundle contracts and implement merge, validate and schema
kind: task
status: open
priority: 2
version: 8
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tg7kbpmm11wycct2vb5n
  - type: blocks
    target: is-01m2f0tj0cf0v7wbz9vbpywjqs
  - type: blocks
    target: is-01m2gbe7t7shfxqxrgfmdmz7yc
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:23.201Z
updated_at: 2026-09-15T00:21:52.654Z
---
UsageSummary, BundleManifest and table record contracts with fixtures, scripts/check_contracts.py and make contracts-check; summary and bundle readers and writers with redaction; bundles as plain *.urollup/ folders of individually zstd-compressed JSONL tables including the captured records table (usage-relevant source records with content and verbose bodies stubbed under a versioned strip policy), with manifest digests over uncompressed tables and atomic publication by staging-folder rename; re-extraction from captured records; mixed raw, summary and bundle input; overlap-safe merge, validate and schema before any totals-only output.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Bundle folder reader hardening: open only manifest-listed files; reject absolute or .. paths, symbolic links, unlisted or missing files, backslashes, drive letters, NUL bytes, case-fold duplicates and file-directory conflicts; verify sizes and digests; bound decompressed bytes (adapted from metaproc safe_archive rules).
- Writers emit explicit nulls for unknown values and quote every timestamp.
- Validation diagnostics never echo values from redacted fields (test).
- Strip policy: stub Pi toolResult.details and compaction retainedTail bodies while keeping nested usage objects; pi-events capture keeps message_end, compaction_end, auto_retry_* and the last message_update usage per message; capture Claude compact_boundary, stop_hook_summary and queue-operation, and Codex task_started, task_complete and timed item_completed lifecycle records.
