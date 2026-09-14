---
type: is
id: is-01m2gcv5af2qkey3bcsswnwre7
title: Review Codex source for authoritative rollout, usage and session formats
kind: task
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - code-review
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T16:43:42.797Z
updated_at: 2026-09-14T17:09:14.213Z
started_at: 2026-09-14T16:43:55.723Z
closed_at: 2026-09-14T17:09:14.212Z
close_reason: "Review complete (notes in scratchpad/reviews/codex.md, 190 pinned links): turn.completed usage is cumulative per thread; token_usage_record since rust-v0.153.0 keyed by response_id; CODEX_THREAD_ID is the subagent thread, hooks get root session_id and no CODEX_* env; exec capture links to rollout by thread_id; copy, compaction and synthetic token_count rules; session_meta.git.branch exists; types and normalizers portable under Apache-2.0."
resolution: null
duplicate_of: null
---
Review the read-only clone attic/codex at 6b9826e (Apache-2.0) as the source of truth for rollout and exec JSON formats: protocol types for session_meta, token_count, token_usage_record, rate_limits, turn events, subagent and fork fields, archived sessions, zstd rollouts, the local thread index, environment variables such as CODEX_THREAD_ID, and hook inputs. Resolve items the research marked unverified (e.g. whether turn.completed usage is per turn or cumulative) and list any fixtures or tests worth reusing.
