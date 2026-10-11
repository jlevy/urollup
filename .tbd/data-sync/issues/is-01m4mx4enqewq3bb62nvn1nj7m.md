---
type: is
id: is-01m4mx4enqewq3bb62nvn1nj7m
title: Claude malformed lines are skipped without a diagnostic
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-11T07:16:45.878Z
updated_at: 2026-10-11T07:16:45.878Z
---
Found in PR #19 review A (A2): the Codex adapter reports malformed-line and pending-tail diagnostics, but the Claude adapter skips an unparseable transcript line without any diagnostic, so a damaged Claude transcript can lose usage while coverage stays complete. Bring Claude to parity with Codex: emit malformed-line for an interior unparseable line and pending-tail for an unfinished last line (design §2.2), with tests and a golden, and decide whether a malformed line that could carry usage makes coverage partial.
