---
type: is
id: is-01m2khsrbqk941a7egfkbzc70p
title: Add statusline command session segment (Candidate)
kind: task
status: open
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
  - candidate
dependencies:
  - type: blocks
    target: is-01m2khtb1x1tjf2mctdsnr8yp4
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-15T22:08:02.934Z
updated_at: 2026-09-15T22:08:22.077Z
---
Milestone 0.5, Candidate (design §9.1 "Statusline Command"; implement only once confirmed): urollup statusline for a Claude Code status line command, per design §6.8.

- Session only from hook input session_id and transcript_path under the --hook-input rules (§6.2); never --latest; exit codes per §6.5.
- Segments: model; session tokens over own and descendant usage, with the list-price estimate and unpriced marker once prices land (0.4); recorded rate_limits used_percentage and resets_at shown as Claude Code reported them (display only, never captured); context from context_window with a percentage only against the recorded context_window_size.
- Claude Code cost.total_cost_usd only as a separately labeled source estimate; no blocks, burn rates or projections (Decision 10).
- Reads only the selected session's sources, no capture cost for the active transcript, no time-based output cache, writes the line only when complete (Claude Code cancels in-flight runs after its 300 ms debounce); fits COLUMNS by dropping trailing segments.
- Proposed gate in the plan's performance targets: p95 under 250 ms on a bench-small session with subagents.
- Phase 2 follow-up: today segment after capture cache reads (separate bead).
