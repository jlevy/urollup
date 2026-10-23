---
type: is
id: is-01m5nfzt66b884714jt1pq0yab
title: Add --agent selection
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
created_at: 2026-10-23T23:01:58.597Z
updated_at: 2026-10-23T23:01:58.597Z
---
Split out of uro-feyo for the alpha: --agent claude|codex (repeatable, union) filters report, daily and sessions to one agent's sources and rows without --no-default-sources gymnastics. The rest of uro-feyo (--sessions-from, --latest, --project, --cwd, --whole-sessions) stays in milestone 0.5. Scope confirmed by the alpha selection decision bead.
