---
type: is
id: is-01m2yn8xk928mrfb1xxxzzj465
title: Land formal GitHub stack 9 for the reviewed 0.1 implementation
kind: epic
status: open
priority: 2
version: 8
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2y7eh0genxhf98p3h08fmn9
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
child_order_hints:
  - is-01m2yn9naybkg326m05qghvg2g
  - is-01m2yn9p87fspx9gs5nmjkc271
  - is-01m2yn9pz2nkednvcskpfnk15r
  - is-01m2yn9qm4pn21f4v9c4s40zzf
  - is-01m2yn9ravz42mpjzswpkcad5y
created_at: 2026-09-20T05:40:24.296Z
updated_at: 2026-09-20T05:40:53.674Z
---
Formal GitHub stack #9: main -> milestone-0.1 (#4) -> codex/v0.1-terminal-ux-acceptance (#8) -> scalable-ingestion (#10) -> ingest-compact (#11) -> ingest-capacity (#12). gh stack sync --remote origin confirms the GitHub stack object and local tracking agree; every layer has the previous head as an ancestor and no rebase is needed. One merge-tracking child per layer must remain open until that PR merges; implementation/finding beads may remain closed because their fixes are complete. Use gh stack for future propagation and landing. Cursor #14 remains a separate planned dialect workstream with integration tracked by uro-knnz. Creating this tracking does not authorize merging or waive milestone performance, QA or policy gates.

## Notes

2026-09-20 verification: gh stack v0.1.0 and the official pinned gh-stack skill are installed. Followed tbd shortcut stacked-prs and official gh-stack sync workflow. GitHub stack #9 synchronized with five open non-draft PRs; local metadata now includes e295b57 at PR12. No head or base changes, no rebase, and no merge occurred. Layer children are ordered by dependency and close only on merge.
