---
type: is
id: is-01m2ksp8dcjmgy7awtpqjjz22v
title: Implement gemini-session and gemini-stream adapters
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2ksqda8y7hfm5pstky74ms1
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-16T00:25:56.907Z
updated_at: 2026-09-16T00:26:34.689Z
---
Phase 2 (design Decision 28, 2026-09-15): Gemini CLI is a planned supported agent, so add tested gemini-session and gemini-stream adapters beside the Pi ones.

Facts come from Gemini CLI v0.60.0 source (commit 733edcb); see the portable brief "Gemini CLI Dialect Facts" and design 2.1, 3.4 and 4.1.

- Discovery: ~/.gemini/tmp, GEMINI_CLI_HOME (reading its .gemini/tmp) and UROLLUP_GEMINI_DIRS; sessions at <root>/<project>/chats/session-<UTC minute>-<first 8 of id>.jsonl, subagents at chats/<parent session id>/<agent id>.jsonl. Identify sessions by content: the bucket also holds logs.json, checkpoint-*.json, checkpoints/, logs/, tool-outputs/, memory/, <session-id>/plans|tasks|tracker, and leftover .unreadable-* and .tmp-<pid> files.
- Project identity: the bucket's .project_root marker, accepted only when its SHA-256 equals the session's projectHash; never decode the slug. Legacy <sha256> buckets are copied, not moved, into the slug bucket, so one session can appear twice.
- Records: only type "gemini" records carry tokens (input, output, cached, thoughts, tool, total). input includes cached, output excludes thoughts, and tool is a tool-use prompt input category. Repeated records of one message id are revisions; the last in file order wins; a null tokens field is not a request.
- Copies: message ids are random UUIDs, so one id in two sessions is a copy owned by the session that recorded it first (migration copy, legacy .json migrated into .jsonl, --session-file import). Messages inside a $set.messages rewrite are revisions, never new requests.
- Rewinds: a $rewindTo record drops messages from the transcript but not from the ledger; keep the usage and add a diagnostic.
- Subagents: spawn edges come from the parent record's toolCalls[].agentId, since the directory name is always the main session even for a nested subagent.
- Coverage gap: only main-chat and subagent turns are recorded. Compaction, routing, loop detection, next-speaker checks, edit correction, summaries and autocomplete, failed attempts before a retry, and remote A2A agents are unobserved and must be reported as a gap, never zero.
- gemini-stream: the init event carries session_id and model; result.stats are whole-process totals that include the unrecorded utility calls, so the session file owns the usage and the capture is only a coverage check.
- Current session: hook input (session_id, transcript_path, cwd) plus the hook-only GEMINI_SESSION_ID. Tool and MCP subprocesses get only GEMINI_CLI=1, so --current without hook input exits 2; an empty transcript_path means recording was off (exit 1); an in-flight request is missing because the assistant record is appended after the stream ends.
- Retention: chats older than general.sessionRetention.maxAge (30d by default) are deleted at startup, so sources reports each root's earliest retained record.
- Strip policies: stub content, displayContent, thoughts and toolCalls args, result and resultDisplay, including inside a $set.messages rewrite, keeping tokens objects verbatim.

Record the Gemini CLI versions each fixture covers. Every fact above is source-derived and unverified against real session files.
