---
type: is
id: is-01m2ksrm1p0ynn8h5135kn3y3a
title: Verify Gemini CLI source-derived format facts against real sessions
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
  - research
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-16T00:27:14.356Z
updated_at: 2026-09-16T00:27:14.356Z
---
Every Gemini CLI fact in the design and the portable brief was read from v0.60.0 source (commit 733edcb) on 2026-09-15; none was checked against real session files, because the maintainer's ~/.gemini data was deliberately not read. Confirm the shapes on the maintainer's own machine, or on a throwaway project, before the Phase 2 rules are settled (portable brief "Next Steps" and "Gemini CLI Dialect Facts").

Check, on a project the maintainer is willing to inspect:

- The metadata line, gemini records, $set and $rewindTo records match the documented shapes, and session and subagent file names match session-<UTC minute>-<first 8 of id>.jsonl and chats/<parent session id>/<agent id>.jsonl.
- Whether totalTokenCount includes toolUsePromptTokenCount: run one turn with a tool-use prompt and compare total against input + output + thoughts + tool. This decides whether ccusage's cached-subtraction heuristic misfires, which is one of the parity ledger entries.
- Whether a legacy .json session is deleted after its .jsonl migration or left beside it, and whether a legacy <sha256> bucket survives beside its slug copy.
- How often $set.messages rewrites and $rewindTo markers occur in normal use.
- Whether a resumed session's later records ever carry a different sessionId than the file's metadata line.
- What an interrupted write leaves behind (torn last line, .tmp-<pid>, .unreadable-<ms>).
- How much usage the unrecorded utility roles account for: run one session with telemetry.outfile set and compare the api_response records by role against the session file's totals and the stream capture's result.stats.

Record the results in the portable brief's Gemini CLI dialect facts, and turn any difference into a fixture case (uro-6mbl) and an adapter change (uro-zogi). Keep prompts, paths and ids out of the brief.
