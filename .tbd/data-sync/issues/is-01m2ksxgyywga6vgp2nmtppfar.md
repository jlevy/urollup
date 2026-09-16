---
type: is
id: is-01m2ksxgyywga6vgp2nmtppfar
title: Add requests and tools commands with observed purpose and tool grouping
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
dependencies:
  - type: blocks
    target: is-01m2ksy3sr8ges0ra56f92ftxg
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:29:55.029Z
updated_at: 2026-09-16T00:30:14.315Z
---
Milestone 0.5: request-level and tool-level reporting. Design §6.3, §6.4, §4.2 and §3.5.

Acceptance:
- requests renders request rows with sizes, ownership status and stable evidence references, honouring --sort and --limit; tools totals tool calls and commands by category.
- --group-by covers purpose from native recorded fields only in Phase 1 (configured rules and --annotation-set are Phase 2, §9.1 Purpose Sources) and tool views.
- Owned, ambiguous and unknown requests each count once in grand totals, and partial candidate selections appear only as possible (§4.2).
- The parity harness's request-attributed explained amounts depend on this command, so its output identifies affected requests by diagnostic or request property.
