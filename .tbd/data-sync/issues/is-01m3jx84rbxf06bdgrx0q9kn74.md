---
type: is
id: is-01m3jx84rbxf06bdgrx0q9kn74
title: Fix Codex paginated fork prefix double counting before alpha acceptance
kind: bug
status: in_progress
priority: 1
version: 6
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
hold: null
hold_until: null
created_at: 2026-09-28T02:24:36.105Z
updated_at: 2026-09-28T03:09:04.783Z
started_at: 2026-09-28T03:07:43.460Z
---
Confirmed at main runtime 4c55617 / docs head 04a3fdd with a synthetic five-line two-file reproduction. Parent cumulative usage is 100 tokens. Child has its own session_meta, parent_thread_id/forked_from_id, history_mode=paginated, multi_agent_version=v2, subagent_history_start_ordinal=3, a copied 100-token token_count at ordinal 1, then cumulative 120 / last 20 at ordinal 3. No foreign session_meta is embedded. Expected parent 100 plus child 20 = combined 120 and two requests. Actual sessions reports parent 100 plus child 120 = report 220 and three requests, copies_excluded=0, coverage.complete=true, no diagnostics. In codex_rollout::observe_parsed_source, active_thread starts at file_thread; native_boundary only switches back to file_thread at/after the boundary. Without a foreign SessionMeta, the pre-boundary prefix never becomes inherited/copy state. Honor explicit paginated boundaries and seed the counter baseline without counting inherited usage as child originals. Prove combined/self totals, one/eight-worker and source-order parity; cover parent-present and parent-missing cases, direct/counter paths, resets and malformed/missing boundary evidence. Preserve conservative uncertainty rather than claiming complete totals when ownership cannot be established. Private-history discrepancies motivated investigation but this bead contains only synthetic values and code reasoning. Re-run private full-history Codex parity after fixing; not every comparator residual has been explained.

## Notes

Delivery layer planned as codex/codex-fork-accounting above codex/usage-analysis-workflow-plan (PR15). Add regression for child-only metadata with explicit ordinal boundary before changing normalization; preserve legacy fallback behavior and cover direct/counter evidence, parent absence and worker/source-order invariance.
