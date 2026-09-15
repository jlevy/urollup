---
type: is
id: is-01m2k28kby8k5nhbf6yxeckedd
title: "Design doc: add a flag index (flag, home section)"
kind: task
status: closed
priority: 3
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-15T17:36:32.125Z
updated_at: 2026-09-15T20:14:31.341Z
started_at: 2026-09-15T19:58:31.379Z
closed_at: 2026-09-15T20:14:31.339Z
close_reason: "Added design §10.4 Flag Index (commit e1eaa27): 51 rows in grouped tables with purpose, home section and plan phase, covering 45 urollup flags and 6 other flags the design names; defined --sort, --limit, --format and --max-input-tokens in §6.4"
resolution: null
duplicate_of: null
---
PR #3 review suggestion S2 (non-blocking). About thirty CLI flags are defined across design sections 2.5, 5.1, 5.2, 5.5, 6.1, 6.4 and 7.3 of docs/urollup-design.md. Add a flag index table (flag, home section) in section 6 or the appendices so implementers can find each flag's rules. Deferred from the PR #3 address pass; do it when the CLI surface candidate (10.1) is confirmed so the list is stable.
