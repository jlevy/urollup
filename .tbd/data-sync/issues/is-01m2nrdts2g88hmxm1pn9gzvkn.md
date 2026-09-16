---
type: is
id: is-01m2nrdts2g88hmxm1pn9gzvkn
title: Reconcile README and all current design docs with tbd documentation guidelines
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - docs
  - design
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-09-16T18:42:21.089Z
updated_at: 2026-09-16T19:08:16.234Z
started_at: 2026-09-16T18:42:30.313Z
closed_at: 2026-09-16T19:08:16.231Z
close_reason: "Audited README and all ten current docs, reconciled status and navigation, added confirmed milestone-0.1 color/progress decisions and acceptance tests, updated PR #4 with the strict 0% documented full-review assessment, and validated locally and in CI at c08986d."
resolution: null
duplicate_of: null
---
Audit and update the top-level README and every current Markdown document under docs/ for factual consistency with the milestone 0.1 implementation and finalized 0.1.0 release plan. Apply tbd common-doc-guidelines throughout, keep README concise and navigational, preserve document ownership boundaries, ensure exact footers and links, format with the pinned flowmark command, and update PR 4. Also assess and report how much of the PR code surface has received full senior code review.

## Notes

Audited README plus all ten current docs using tbd/common-doc guidance. Edited README, design, main plan and publishing plan; six historical research briefs were already structurally compliant. Added confirmed milestone-0.1 Decisions 29 (terminal-aware color) and 30 (interactive progress), updated readiness gates, fixed a broken link, and added complete navigation. Validation: pinned Flowmark format/check, git diff --check, H1/footer checks, and local link/anchor validation passed. PR #4 strict independent full implementation review coverage is 0% documented; tracked by uro-nncx. Color and progress implementation are tracked by uro-nazs and uro-wqf8.
