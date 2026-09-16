---
type: is
id: is-01m2ksx3pmqbfk86av545tdvab
title: Add pricing staleness diagnostics and --require-priced
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
dependencies: []
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-16T00:29:41.435Z
updated_at: 2026-09-16T00:29:41.435Z
---
Milestone 0.4: make pricing coverage explicit. Design §4.5, §6.4 and §6.5.

Acceptance:
- A staleness diagnostic appears when a report window ends more than 90 days after the table review date, because that release cannot know later price changes.
- Reports show priced, default-assumed and unpriced request and token counts, as the §6.6 examples do.
- --require-priced exits 3 when any tokens are unpriced; a non-strict run with pricing gaps still exits 0 with the gaps in its coverage fields.
- Diagnostics go to stderr and appear in JSON coverage fields; goldens cover each case.
