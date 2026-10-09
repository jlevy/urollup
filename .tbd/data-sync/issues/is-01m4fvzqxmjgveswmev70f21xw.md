---
type: is
id: is-01m4fvzqxmjgveswmev70f21xw
title: "PR #16 B1: nested paginated subagents make reconciled histories partial"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:30.771Z
updated_at: 2026-10-09T08:20:30.771Z
---
Medium. Review B https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389. codex_rollout.rs:1009-1013, :1522-1530. R7 (root->child->grandchild, all originals present) counts 126 correctly but reports Partial({CopyWithoutOriginal}) with no diagnostic, and the child's self selection is Partial.
