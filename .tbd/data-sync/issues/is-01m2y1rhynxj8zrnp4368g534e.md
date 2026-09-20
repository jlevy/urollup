---
type: is
id: is-01m2y1rhynxj8zrnp4368g534e
title: Add Cursor current-session detection
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels: []
dependencies: []
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
created_at: 2026-09-19T23:59:25.139Z
updated_at: 2026-09-19T23:59:25.139Z
---
Map the research brief environment or hook signals onto CurrentEnvironment and Agent::Cursor. Until then, a detected Cursor session exits 2 with an unsupported-dialect diagnostic, as Pi does in 0.1. The only heuristic remains --latest and is never implicit.
