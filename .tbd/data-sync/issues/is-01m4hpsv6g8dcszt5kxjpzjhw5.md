---
type: is
id: is-01m4hpsv6g8dcszt5kxjpzjhw5
title: Codex guardian-review subagent usage is excluded as copied history
kind: bug
status: in_progress
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies:
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-10T01:28:23.503Z
updated_at: 2026-10-10T17:45:37.004Z
---
Found validating PR #26 on real history (local, aggregate only). Codex guardian_review subagent rollouts (session_meta thread_source guardian_review, history_mode paginated) can declare subagent_history_start_ordinal past the file's last line ordinal, so PR #16's boundary rule treats every line, including every token_usage_record, as copied parent history and excludes it. Those records name the subagent's own thread (payload.thread_id == session_meta.id), and their response IDs do not appear in the parent rollout, so they are the subagent's own requests: the usage is silently dropped with coverage complete. In the maintainer's archive this affects thousands of guardian rollouts and a few percent of Codex tokens; guardian rollouts whose boundary falls inside the file, and ordinary subagents, keep their own records after the boundary and are counted. Proposed rule: a token_usage_record whose thread_id names the file's own thread is that thread's original wherever it sits; a record naming another thread is a copy; the declared boundary decides only lines that name no thread (counters, compacted). A counter twin follows its twin record's owner and role. Needs a synthetic fixture of the guardian shape, design §3.4 and PR #16 plan text updates, and a real-history rerun. Related: PR #26 round 2 keys copied counters to their twin's response ID, which surfaces this as conflicting-owners diagnostics between the guardian thread and its parent.

## Notes

PR #32 round 2 at e5a29ca (rebased on main a4bad34 with PR #26): records own by thread_id; unnamed lines follow the latest naming line; migrated counter-only prefixes decided by parent-root turns and counter-only parent totals; undecided unnamed usage is a gap only where the region shows migrated content; a compacted record naming its own thread with no other record counts, and a later total beyond the child's records is a gap. B3 deferred to uro-p9ua. Needs a real-history rerun.
