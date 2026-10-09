---
type: is
id: is-01m2ksms2rd2kwm66zpy8p159d
title: Decide how a request observed only as copies is reported
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - design-decision
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:25:08.437Z
updated_at: 2026-10-09T08:58:15.437Z
---
Design 3.3 says a copy nested inside another record never counts, and 2.1 says usage that never reaches local logs is an unobserved coverage gap rather than zero. A request whose only observations are copies sits between the two: the usage was really consumed, the original file is gone (Claude Code deletes transcripts after cleanupPeriodDays, and a Codex parent rollout can be archived away), and the copy states what it was.

uro-spce records such a request with Counting::CopyOnly, a copy-without-original diagnostic and no counted usage, and keeps the copies as evidence. Decide whether that stays, or whether a copy-only request should also raise a coverage gap so reports show the missing usage rather than only a diagnostic. See crates/urollup-core/src/ledger/reconcile.rs and the test a_request_seen_only_as_copies_counts_nothing_and_is_diagnosed.

## Notes

Alpha stabilization adopts conservative copy-only reporting: do not count copied usage without an original; preserve the copy evidence and diagnostic, and mark whole-history totals incomplete. Known selected owners with copy-only requests are also incomplete. This is an implementation choice under the maintainer alpha goal, not a claim of separately recorded policy approval. Changes under uro-kpbp are in progress and require full gates before closure.

2026-10-09, PR #16 reviews A (https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467191112) and B (https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389), both at 3c4e9cd: the user decided to revert the copy-only coverage rule from PR #16 and keep this decision open here. PR #16 commit 23b0119 restores the earlier semantics: copy-only requests are excluded from counted totals and do not change completeness, for whole-history totals and for selections; the copy-without-original diagnostic is unchanged (Claude remaps it; Codex still drops it). The uro-kpbp double-count fix stays. Evidence the rule fired on shapes where nothing is missing, all synthetic:
- A1 (High): a 0.153+ legacy-destination forked subagent with its parent present (P4) and a paginated child with boundary 2 whose prefix holds only the parent's copied token_count (P5) both counted 120 tokens correctly but reported complete=false with no diagnostic. In direct-usage files a copied token_count is keyed only by the owner's last response ID, which 0.153+ prefixes no longer carry, so the copy can never reconcile.
- B1 (Medium): nested paginated subagents root -> child -> grandchild with every original present (R7) counted [100, 20, 6] = 126 correctly, but the root's counter copied through the child's prefix is keyed to the child, so one request stayed copy-only and whole-history coverage and the child's self selection were partial.
- A3 (Medium): with the rule, a Codex report could say partial with no visible reason, because the Codex adapter drops copy-without-original diagnostics and CoverageSummary carries no reasons (P1: complete=false, diagnostics=[]).
- A4 (Medium): the rule changed Claude coverage too (progress-nested-subagent golden flipped to partial) inside a Codex fix, ahead of this decision; the Completeness::Complete doc no longer listed every condition.
Options the reviews raised if the rule returns: do not emit observations for copies that can never be keyed; key copies by owner thread plus counter signature (or one key per known ancestor for nested prefixes); restrict the rule to copy-only requests whose owner is unknown or undiscovered; and in every case give partial coverage a visible reason (a Codex copy-without-original remap or reasons in CoverageSummary). PR #16 now pins the current behavior with tests an_unkeyed_copied_token_count_with_its_parent_present_keeps_coverage_complete and nested_paginated_subagents_with_every_original_present_report_complete_coverage in crates/urollup-core/tests/paginated_forks.rs; changing the policy should change those tests deliberately.
