---
type: is
id: is-01m3jx84rbxf06bdgrx0q9kn74
title: Fix Codex paginated fork prefix double counting before alpha acceptance
kind: bug
status: in_progress
priority: 1
version: 14
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies:
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
  - type: blocks
    target: is-01m2ke36qgfvdvnhw7c7v5esm6
  - type: blocks
    target: is-01m3jzhnnk1xk472z6y6gfj5be
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
child_order_hints:
  - is-01m4fvzmya1w12qgt45dt2929v
  - is-01m4fzvd803vx5er2c7k9cycgn
hold: null
hold_until: null
created_at: 2026-09-28T02:24:36.105Z
updated_at: 2026-10-09T09:28:03.069Z
started_at: 2026-09-28T03:07:43.460Z
---
Confirmed at main runtime 4c55617 / docs head 04a3fdd with a synthetic five-line two-file reproduction. Parent cumulative usage is 100 tokens. Child has its own session_meta, parent_thread_id/forked_from_id, history_mode=paginated, multi_agent_version=v2, subagent_history_start_ordinal=3, a copied 100-token token_count at ordinal 1, then cumulative 120 / last 20 at ordinal 3. No foreign session_meta is embedded. Expected parent 100 plus child 20 = combined 120 and two requests. Actual sessions reports parent 100 plus child 120 = report 220 and three requests, copies_excluded=0, coverage.complete=true, no diagnostics. In codex_rollout::observe_parsed_source, active_thread starts at file_thread; native_boundary only switches back to file_thread at/after the boundary. Without a foreign SessionMeta, the pre-boundary prefix never becomes inherited/copy state. Honor explicit paginated boundaries and seed the counter baseline without counting inherited usage as child originals. Prove combined/self totals, one/eight-worker and source-order parity; cover parent-present and parent-missing cases, direct/counter paths, resets and malformed/missing boundary evidence. Preserve conservative uncertainty rather than claiming complete totals when ownership cannot be established. Private-history discrepancies motivated investigation but this bead contains only synthetic values and code reasoning. Re-run private full-history Codex parity after fixing; not every comparator residual has been explained.

## Notes

PR16 at 3c4e9cd is ready for review above PR15; all 15 hosted CI jobs pass. Local full make check passed including all 30 negative gate probes; final spec-status prose separately passed pinned formatting. Latest release full-history report completed under watchdog with incomplete coverage explicit. One real Claude and Codex session each exactly matched whole-machine session rows. Controlled whole-history QA, cost support and process-wide memory safety remain alpha acceptance work, not claims of this PR. GitHub stack #17 is verified. No merge performed.
