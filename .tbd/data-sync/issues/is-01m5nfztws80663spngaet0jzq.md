---
type: is
id: is-01m5nfztws80663spngaet0jzq
title: State each command's default selection and explain empty or failed runs
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - cli
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:59.319Z
updated_at: 2026-10-23T23:01:59.319Z
---
First-run papercuts from the 2026-10-23 review: report without flags is a usage error outside an agent session and help never says report defaults to --current while daily and sessions default to --all, or that --source switches report to --all (cli.rs:1161-1164); with no logs found, commands print an empty table and exit 0 with no hint (missing default roots are skipped silently, claude_project.rs:604); I/O errors print Debug kinds such as PermissionDenied and InvalidInput, and an open error prints the path twice (adapters.rs:131, cli.rs:1024-1028); progress is one static line on long runs; --max-rows help is jargon. Fix the help text, add a 'no Claude Code or Codex logs found under …' hint, and print readable errors.
