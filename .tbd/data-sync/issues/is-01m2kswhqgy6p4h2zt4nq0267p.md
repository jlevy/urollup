---
type: is
id: is-01m2kswhqgy6p4h2zt4nq0267p
title: Implement retained reads and retained versions for deleted and rewritten sources
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.3
  - capture-cache
dependencies:
  - type: blocks
    target: is-01m2kswpq22hpbr9b3cx7rbht3
  - type: blocks
    target: is-01m2kswtgsj4hcyk1x890reavj
parent_id: is-01m2ke4qtjw7gzyjmmzxxs64mf
created_at: 2026-09-16T00:29:23.052Z
updated_at: 2026-09-16T00:29:32.056Z
---
Milestone 0.3: keep usage after agents delete or rewrite their logs, which is why the store exists (Claude Code deletes transcripts after cleanupPeriodDays, 30 by default). Design §2.5 and §2.2.

Acceptance:
- When a source log is gone, runs read its captured records instead and report the source as retained with its capture time; until the Phase 2 cache, runs read captured records only for such sources.
- When the prefix check shows a source was replaced, truncated or rewritten in place, the previous entry is kept as a retained version.
- A rewrite that changes the source's first complete record, such as a Codex rollout migration or a Pi v1 or v2 file rewritten on load, yields a new src- ID; the old entry becomes retained under its own ID, runs read both, and reconciliation deduplicates their shared observations by analytical ID, so a rewrite that drops records never silently drops usage.
- Usage of Codex rolled-back turns that a migration drops keeps counting through the retained entry, because failed and retried requests consume the usage they report.
- A .jsonl rollout and its .jsonl.zst twin with the same thread and rollout ID are one logical source whose representation changed, not two; a path briefly absent while another tool rewrites it is a mid-scan change, not a deletion.
