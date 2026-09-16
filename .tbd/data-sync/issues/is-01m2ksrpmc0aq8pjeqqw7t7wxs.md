---
type: is
id: is-01m2ksrpmc0aq8pjeqqw7t7wxs
title: Decide whether to read Gemini CLI telemetry for its unrecorded calls (Candidate, later)
kind: feature
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - later
  - candidate
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-16T00:27:17.000Z
updated_at: 2026-09-16T00:27:17.000Z
---
Later, candidate (raised by the 2026-09-15 Gemini CLI review; design 2.1 lists Gemini OpenTelemetry exports as an unsupported format).

Gemini CLI records only main-chat and subagent turns in its session files. Every other model call it makes (compaction, routing, loop detection, next-speaker checks, edit correction, session summaries, autocomplete) and every failed attempt before a retry reach only telemetry, where logApiResponse emits one record per call with input, output, cached, thoughts, tool and total token counts, the model, prompt_id, an LlmRole, gen_ai.response.id and a session.id attribute. A gemini-session rollup is therefore a lower bound by construction.

Decide whether urollup should read a Gemini telemetry export as its own dialect, and if so:

- Telemetry is off by default and has no default path; it needs telemetry.outfile or GEMINI_TELEMETRY_OUTFILE, so such a file exists only for users who opted in, and discovery would be through --source or the source manifest, never a default root.
- The file exporter appends pretty-printed JSON objects rather than one record per line, so the reader cannot be a plain JSONL reader.
- Records carry the signed-in Google account's email as a common attribute, so the dialect needs its own capture and export strip policies (design 2.4) before any of it is captured.
- Response ids exist here and nowhere else, so telemetry records would be the only way to key Gemini requests and to reconcile them with session records and with a stream capture's process totals.
- The alternative is to keep reporting the utility calls as an unobserved coverage gap, which is what Decision 28 does today.

Depends on the Gemini adapters (uro-zogi) landing first; it only makes sense as a second source for the same sessions.
