---
type: is
id: is-01m4fvzqxmjgveswmev70f21xw
title: "PR #16 B1: nested paginated subagents make reconciled histories partial"
kind: bug
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:30.771Z
updated_at: 2026-10-09T09:05:34.691Z
closed_at: 2026-10-09T09:05:34.690Z
close_reason: "fixed in 23b0119: nested paginated subagents report complete; pinned by nested_paginated_subagents_with_every_original_present_report_complete_coverage; ancestor keying deferred to uro-xpd0. Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6077861115"
resolution: null
duplicate_of: null
---
Medium. Review B https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389. codex_rollout.rs:1009-1013, :1522-1530. R7 (root->child->grandchild, all originals present) counts 126 correctly but reports Partial({CopyWithoutOriginal}) with no diagnostic, and the child's self selection is Partial.
