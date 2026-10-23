---
type: is
id: is-01m4pbcgp1px9e1bnjhtf6g4fv
title: Claude fork replays still decide spawn parents once the session transcript is deleted
kind: bug
status: open
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies:
  - type: blocks
    target: is-01m5nfwqwjgaf1665db3fhvaan
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-11T20:45:04.577Z
updated_at: 2026-10-23T23:02:38.095Z
---
Found in PR #34 review C (C2, Low; better than before PR #34, not a regression). crates/urollup-core/src/adapters/claude_project.rs: every record that is not a forced copy registers its tool uses, and thread_graph picks a spawning tool use's issuer by owner precedence with a self-spawn guard. While S.jsonl exists its main-session record wins. Once S.jsonl is deleted (S/ remaining), other forks' replays are the only candidates: sibling forks (S spawns F1 with record B and later F2 with record C; F2 replays S through C) give F1 <- F2, a wrong parent; nested forks (F1 spawns F3, F3 replays F1 including S's record B) give F1 <- F3 and F3 <- F1, a 2-cycle, and --session S then selects only main(S), so both forks' requests leave S's subtree. Totals unchanged (5 owned, 0 ambiguous). Repro: fork-subagent-uuid-replay with fork sidecars added, 1 and 8 workers, both orders. Unverified: whether Claude Code lets a fork fork, and how often S.jsonl is deleted while S/ remains. Fix direction (review C option a): in decode_source, once the sidecar's own tool use is known, drop the source's tool-use registrations at or before the record that reports it (that prefix replays the parent's history); optionally break any spawn cycle in thread_graph by re-parenting its members to their session.
