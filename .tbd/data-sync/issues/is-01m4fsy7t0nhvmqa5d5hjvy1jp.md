---
type: is
id: is-01m4fsy7t0nhvmqa5d5hjvy1jp
title: "PR #14 A9: facet table lists unimplemented summary properties.agent"
kind: bug
status: closed
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsxxjgrahm03vd1fr1vhms
hold: null
hold_until: null
created_at: 2026-10-09T07:44:44.351Z
updated_at: 2026-10-09T07:50:32.762Z
started_at: 2026-10-09T07:45:11.211Z
closed_at: 2026-10-09T07:50:32.761Z
close_reason: "Fixed in 712016e: the facets-today table marks summary properties.agent as designed (§5.2), not implemented, and the Facet Contract agent row says it applies once the summary artifact exists. Confirmed by pinned flowmark --auto --check . (exit 0), git diff --check (clean), and the relative link and anchor check of the four changed files (907 links, bad=0)."
resolution: null
duplicate_of: null
---
Severity: Low. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: plan-2026-09-19-cursor-dialect.md :128-134 ("How Claude Code and Codex Set Facets Today" table), :203 (Facet Contract agent row).

Problem: the "today" table lists summary properties.agent beside implemented types, but no summary artifact exists at this head (crates/urollup-core/src/artifacts.rs holds only a module comment; properties.agent appears only in the design's §5.2 contract example).

Fix: mark it as designed (§5.2 summary contract), not implemented.
