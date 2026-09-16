---
type: is
id: is-01m2kt252w8nytk407sj8p30ta
title: Implement the capture strip policy with unknown-key diagnostics
kind: task
status: closed
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.3
  - capture-cache
dependencies: []
parent_id: is-01m2ke4qtjw7gzyjmmzxxs64mf
created_at: 2026-09-16T00:32:26.715Z
updated_at: 2026-09-16T03:01:53.978Z
closed_at: 2026-09-16T03:01:53.977Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-n8qu is the original.
resolution: duplicate
duplicate_of: is-01m2kswb5katd7wnv9sevd37pw
---
Milestone 0.3: the versioned per-dialect capture policy that decides what a captured record keeps. Design §2.3, §2.4 and Decision 7.

Acceptance:
- Captured records keep every record carrying usage, identity, model, effort, timing, turn lifecycle, subagent or fork linkage, tool call structure and outcome, or provider limits. That includes Claude system compact_boundary, stop_hook_summary and queue-operation records, Codex task_started, task_complete and timed item_completed events, and Claude progress records nesting a subagent assistant message. Records with none of these, such as streaming display events, are counted in the entry manifest but not kept.
- Known content fields (prompt and response text, reasoning text, tool arguments and results, hook output, attachments, images, file snapshots and injected context) become stubs recording the field's byte length and a keyed HMAC-SHA-256 digest under a random key kept owner-only in the store, so content size and repetition stay measurable without the content.
- Payloads that embed other messages, such as Pi toolResult.details and compaction retainedTail, are stubbed while the usage objects inside them stay verbatim as evidence.
- Values under keys the policy does not recognize stay verbatim so extraction can be rerun for newly discovered fields, and each capture reports those keys per dialect version as a diagnostic.
- The policy version is recorded in every entry manifest, and a later adapter version applies to old captured records without the logs (§2.3 re-extraction).
- A structural command summary computed before tool arguments are stubbed is a queued review decision (§9.2) and is out of scope until confirmed in uro-gxen.
