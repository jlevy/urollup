---
type: is
id: is-01m2kt21w66490bxt0vr6kxsra
title: Add the export command with --per-session and mixed artifact input
kind: task
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2kt23gtg5e014w43p0a46c9
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:32:23.427Z
updated_at: 2026-09-16T03:01:52.729Z
closed_at: 2026-09-16T03:01:52.727Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-cye3 is the original.
resolution: duplicate
duplicate_of: is-01m2ksw5c3g28x3wk3g9hhfybr
---
Milestone 0.2: the export command over the summary and bundle writers. Design §5.1, §6.3 and §6.4.

Acceptance:
- `export --format summary` writes one summary with one extent per thread; `--per-session --output-dir` writes one summary per selected top-level session with its descendants under the selected scope (--per-session is Candidate, §9.1 CLI Surface).
- `export --format bundle` includes captured records by default, with --no-records to omit them, and applies the export strip policy and redaction profile.
- Raw JSON and JSONL logs, compressed logs, summaries and bundles all enter the same reconciliation pipeline through one --source flag and can be mixed; every command that accepts a summary accepts a bundle; there is no --input flag.
- --output files and bundles are published atomically; a JSON document is written only after the query completes; cancellation writes no document or completion record and publishes no file.
- JSON and CSV report rows carry no session extents and are never merge inputs.
- CLI goldens for each form, with exit codes per §6.5.
