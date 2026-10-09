---
type: is
id: is-01m4fsy5zxhtfskdjgn686rb1b
title: "PR #14 A4: define the Cursor request key and multi-bubble token rule"
kind: bug
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: null
labels: []
dependencies:
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-10-09T07:44:42.491Z
updated_at: 2026-10-09T07:50:57.038Z
started_at: 2026-10-09T07:45:09.617Z
---
Severity: Medium. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Deferred from PR #14 (planning gap). Blocks the adapter bead uro-p9ay.

Where: plan-2026-09-19-cursor-dialect.md Locked Format Facts turn/request row (:174) and identity keys (:233-238); research-2026-09-19-cursor-agent-logs.md :140-142 (904 of 8,000 sampled bubbles with requestId and modelInfo; 5,537 with usageUuid), :199-201, :247-251 (nonzero tokenCount on 297 of 8,000).

Problem: the plan treats requestId and usageUuid as request IDs but does not say which is the request key, whether one bubble can carry both, or how several bubbles of one request (assistant text, thinking, tool steps) combine their tokenCount. If Cursor repeats a request's tokens on more than one bubble, summing overstates usage. The Claude Code adapter needed an explicit rule for the same shape (several records per message.id repeat usage).

Decide and record in the plan:
- The request key and its identity scope.
- The combining rule for bubbles that share it (sum, last or max), chosen from fixture evidence.
- A diagnostic when bubbles of one request disagree.
- Test cases for a multi-bubble request with repeated token counts and with split token counts.

## Notes

Deferred from PR #14 review A (tracking parent uro-l2tq). Listed in plan-2026-09-19-cursor-dialect.md under Open Questions Before Implementation at 712016e; blocks uro-p9ay. Waits on a design pass before adapter implementation; PR #14 is docs-only.
