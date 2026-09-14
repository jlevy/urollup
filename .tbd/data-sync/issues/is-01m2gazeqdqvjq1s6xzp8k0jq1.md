---
type: is
id: is-01m2gazeqdqvjq1s6xzp8k0jq1
title: Add windows report over recorded provider limits
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T16:11:06.348Z
updated_at: 2026-09-14T19:58:19.300Z
---
windows report grouping usage by recorded provider limit and window (Codex rate_limits exact; Claude quotaLimits with configured, labeled window length), latest recorded used_percent, uncovered usage, and local totals labeled partial because limits are shared with surfaces absent from local logs. No inferred ccusage-style blocks.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Keep unrecognized rateLimitType values; never infer windows from endpoints or error text; deduplicate identical consecutive Codex snapshots and mark carried-forward plan_type and credits as possibly stale.
- Pending decision: grouping windows by a configured organization or quota group.
