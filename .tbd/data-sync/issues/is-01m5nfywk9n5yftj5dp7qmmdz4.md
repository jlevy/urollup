---
type: is
id: is-01m5nfywk9n5yftj5dp7qmmdz4
title: Treat an unreadable Claude subagent sidecar as lost linkage, not a fatal error
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:28.297Z
updated_at: 2026-10-23T23:01:28.297Z
---
A truncated or unreadable agent-*.meta.json (claude_project.rs:679, 708-722) fails every command with 'cannot parse Claude subagent metadata' until the user finds and deletes the file; a sidecar read while Claude Code writes it can plausibly do the same. The sidecar only carries agentType and toolUseId. Fix: lose only the subagent linkage, with a diagnostic, and keep counting the transcript. Coordinate with uro-zd3m, which vets the sidecar open.
