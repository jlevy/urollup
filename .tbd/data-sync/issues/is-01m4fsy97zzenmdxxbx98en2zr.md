---
type: is
id: is-01m4fsy97zzenmdxxbx98en2zr
title: "PR #14 A13: define Cursor opt-in override entries, source targets and JSONL-only outcome"
kind: bug
status: open
priority: 3
version: 6
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: null
labels:
  - post-0.1
dependencies:
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-10-09T07:44:45.822Z
updated_at: 2026-10-11T06:29:17.139Z
started_at: 2026-10-09T07:45:12.480Z
---
Severity: Low. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Deferred from PR #14 for the design part; blocks the adapter bead uro-p9ay. The factual part (Linux and Windows locations are conventional and not verified, matching the brief's Methodology) is fixed on PR #14.

Where: plan-2026-09-19-cursor-dialect.md Goals (:42-45) and Discovery Roots (:245-257); research-2026-09-19-cursor-agent-logs.md :507-508.

Decide and record in the plan:
- What one UROLLUP_CURSOR_DIRS entry names: the User directory, globalStorage, the state.vscdb file, or ~/.cursor.
- How one variable covers two root classes that live in different trees (application user state versus ~/.cursor/projects transcripts), or whether two variables are needed.
- The --source target for each root class.
- What JSONL-only input does (rejected, or sessions with unknown usage), given that JSONL is not a usage owner.

## Notes

Deferred from PR #14 review A (tracking parent uro-l2tq). Factual part fixed in 712016e: Linux and Windows locations marked as conventional and unverified. Remaining design questions are listed in plan-2026-09-19-cursor-dialect.md under Open Questions Before Implementation; blocks uro-p9ay.
