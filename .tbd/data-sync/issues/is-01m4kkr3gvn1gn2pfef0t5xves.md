---
type: is
id: is-01m4kkr3gvn1gn2pfef0t5xves
title: Seeded bounded-migrated Codex child reports its seed as a dropped-response gap
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-10T19:13:29.626Z
updated_at: 2026-10-10T19:13:29.626Z
---
Found in PR #32 review C (C5). A Codex child with usage records whose rollout a bounded migration rewrote starts from a running total seeded with its parent's total. PR #32's dropped-response check compares that first running total with what the child's own records account for, so the seed reads as dropped usage and the child reports a codex-history-boundary-unverified gap even when nothing was dropped. Totals are correct; only coverage is pessimistic. Discounting the seed needs the parent's counter totals, but roots with usage records are observed on their decode worker and drop their records, so a fix must retain a per-root counter-total digest across the history and make such children wait for normalization. Pinned by a_seeded_bounded_child_with_records_reports_its_seed_as_a_gap in paginated_forks.rs.
