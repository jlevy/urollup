---
type: is
id: is-01m3jx84rbxf06bdgrx0q9kn74
title: Fix Codex paginated fork prefix double counting before alpha acceptance
kind: bug
status: closed
priority: 1
version: 16
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
updated_at: 2026-10-09T17:33:07.487Z
started_at: 2026-09-28T03:07:43.460Z
closed_at: 2026-10-09T17:33:07.486Z
close_reason: "Merged to main in bdaa7ce via PRs #15 and #16 (stack #17) after senior, correctness, follow-up and restack reviews with every finding dispositioned; real-history run on the merged tree verified."
resolution: null
duplicate_of: null
---
Confirmed at main runtime 4c55617 / docs head 04a3fdd with a synthetic five-line two-file reproduction. Parent cumulative usage is 100 tokens. Child has its own session_meta, parent_thread_id/forked_from_id, history_mode=paginated, multi_agent_version=v2, subagent_history_start_ordinal=3, a copied 100-token token_count at ordinal 1, then cumulative 120 / last 20 at ordinal 3. No foreign session_meta is embedded. Expected parent 100 plus child 20 = combined 120 and two requests. Actual sessions reports parent 100 plus child 120 = report 220 and three requests, copies_excluded=0, coverage.complete=true, no diagnostics. In codex_rollout::observe_parsed_source, active_thread starts at file_thread; native_boundary only switches back to file_thread at/after the boundary. Without a foreign SessionMeta, the pre-boundary prefix never becomes inherited/copy state. Honor explicit paginated boundaries and seed the counter baseline without counting inherited usage as child originals. Prove combined/self totals, one/eight-worker and source-order parity; cover parent-present and parent-missing cases, direct/counter paths, resets and malformed/missing boundary evidence. Preserve conservative uncertainty rather than claiming complete totals when ownership cannot be established. Private-history discrepancies motivated investigation but this bead contains only synthetic values and code reasoning. Re-run private full-history Codex parity after fixing; not every comparator residual has been explained.

## Notes

2026-10-09: PR #16 is at 1bec262, rebased onto PR #15 at e17d478 and still layer 2 of stack #17. Reviews A and B (at 3c4e9cd) and follow-up review C (at 5d8cdeb, verdict approve, six Low findings) are addressed with marked disposition replies (parent beads uro-1ker and uro-e91e). Changes since 3c4e9cd: the copy-only coverage rule was reverted by the user's decision (policy open in uro-xpd0); fork-boundary anomalies degrade per rollout with a codex-history-boundary-unverified diagnostic and a coverage gap instead of aborting; the first own counter step is checked against last_token_usage for every explicit boundary (including 0) and after copied counters, and totals that only repeat the running total count as no usage; frozen cases paginated-counter-prefix and unverified-fork-boundary with goldens and parity were added. Hosted CI run 37908179089 at 5d8cdeb: 14 of 15 jobs passed; Dependency audit fails only on GHSA-vfj7-8cjw-p6xm until the stack is synced onto a main that contains PR #18. Deferred follow-ups: uro-h2sf (B5), uro-omlf (C2), uro-akgu (C4), uro-x3sk (C6). Earlier evidence at 3c4e9cd (local make check with 30 gate probes, private whole-machine runs, two real sessions matching their whole-machine rows) was not repeated for the current head. Controlled whole-history QA, cost support and process-wide memory safety remain alpha acceptance work. No merge performed.
