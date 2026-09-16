---
type: is
id: is-01m2kspssne0kaf0ykdmt0pbe3
title: Fold the observed Claude transcript shapes into the research brief and adapter
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:26:14.707Z
updated_at: 2026-09-16T08:12:04.181Z
closed_at: 2026-09-16T08:12:04.180Z
close_reason: "Implemented in e4965f1: the Claude project adapter records apiBlockIndex as the native block sequence and uses it before file position for equal-output selection; unit tests cover that tie-break and prove ordinary message iterations are not added twice. The portable research brief now records the complete observed quotaLimits shape and values, block-record omissions, iteration behavior, mixed transcript record types and keys, and expanded subagent metadata; the fixture research notes no longer call quotaLimits unverified. Markdown was formatted with the pinned Flowmark command and CARGO_INCREMENTAL=0 make check passes."
resolution: null
duplicate_of: null
---
A 2026-09-15 scan of a real Claude Code 2.1.270 session (recorded in the fixtures README's research notes) settled shapes the portable research brief leaves open or omits. Fold the ones that change behavior into docs/project/research/research-2026-09-13-portable-agent-usage.md and the claude-project adapter (uro-y2qj):

- quotaLimits is an object with status, rateLimitType, resetsAt, isUsingOverage, overageStatus, overageDisabledReason and unifiedRateLimitFallbackAvailable; observed values include status and overageStatus 'rejected', rateLimitType 'five_hour' and 'seven_day', overageDisabledReason 'org_level_disabled_until' and 'out_of_credits'. The brief lists three keys and calls the shape unverified.
- apiBlockIndex numbers each content-block record within its response, so block records are identifiable without comparing content.
- A streaming block record can omit output_tokens_details, server_tool_use, iterations and speed while a later record of the same response has them: more evidence for selecting one whole record.
- iterations appeared on nearly every record with a single message element equal to the top-level usage.
- Other record types a session file carries: attachment (environment, model, skill_listing, instructions, session_context, date, deferred_tools_delta, prompt_snapshot, remote_session_change, total_tokens_reminder), file-history-snapshot, last-prompt, custom-title, agent-name, mode, atis-latch, pr-link, bridge-session, queue-operation.
- Other keys: entrypoint, promptId, effort and perTurnEffort at the record level, wireToolInputs keyed by tool-use ID, sourceToolAssistantUUID on tool results, and container, stop_details, diagnostics and context_management inside message.
- Subagent .meta.json also carries worktreePath, worktreeBranch, spawnedWithWorktree, requestShape and requestNonInteractive.
