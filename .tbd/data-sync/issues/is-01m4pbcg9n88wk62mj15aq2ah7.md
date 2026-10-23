---
type: is
id: is-01m4pbcg9n88wk62mj15aq2ah7
title: Key Claude selection families by session ID so path and agent selectors reach copies across projects and roots
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
created_at: 2026-10-11T20:45:04.181Z
updated_at: 2026-10-23T23:02:38.090Z
---
Found in PR #34 review C (C1, Low, mostly pre-existing). crates/urollup/src/cli.rs builds a Claude subagent copy's named family as a sibling path (family.with_file_name(&named)). After PR #34, only the named session's native and thr- IDs reach a copy whose session S lives in another project directory or root. Two selector kinds then read a different file set from whole history: (1) the path of S's transcript omits the copy; (2) the copy itself by agent ID, path or thr- ID does not read S's transcript, so a fork-style or /btw copy's uuid replays become its own originals and over-count. Repro (hermetic): copy claude-project/fork-subagent-uuid-replay, resume S as R in projects/-Users-example-other/ (R.jsonl a byte copy of S.jsonl, R/ a copy of S/), delete S/: sessions --session a0000000000050001 gives S/a... 3 requests 10,469 tokens against whole history's 1 request 4,174; sessions --session <S's project>/<S>.jsonl omits the fork. A narrowing probe found 15 such disagreements across cross-project and two-root layouts (main: 11 of the same 15). Severity rests on whether Claude Code writes such layouts at all; the aggregate counts in uro-mxv0 (PR #34 B6) will size it. Fix direction (review C option a): key Claude families by session ID (located or named session) rather than path, so S's transcript and directory form family S wherever they lie and a copy joins R and S; extend exact_selection_reads_a_copied_subagent_whose_session_lies_elsewhere with the fork fixture and the agent-ID and S-path selectors.
