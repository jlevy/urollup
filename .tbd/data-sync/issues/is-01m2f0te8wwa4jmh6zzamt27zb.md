---
type: is
id: is-01m2f0te8wwa4jmh6zzamt27zb
title: Freeze sanitized fixtures and representative corpus manifest
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
dependencies:
  - type: blocks
    target: is-01m2f0tewzgw6zd3vzrp2c865s
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:21.852Z
updated_at: 2026-09-14T19:58:20.549Z
---
Public synthetic or sanitized fixtures, including the research brief's double-counting cases, plus a consented representative corpus manifest.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- ccusage negative fixtures: nested null in tool input, sidechain replay before its parent, gateway message-ID reuse, single-event compressed Codex replay, fork events under 1 s apart, microsecond timestamps, codex-auto-review model, cache_creation disagreeing with the flat count; adapt ccusage's fs_fixture! macro and relevant tests (MIT notice).
- Codex synthetic goldens: cumulative exec usage over resume, context-full fills, compaction estimates, info:null, multi-limit_id snapshots, legacy user fork copies, legacy and paginated subagent prefixes, revert files, .zst twins, archived renames, migrated rollouts; port squares' synthetic Codex builders (record squares repo and commit).
- Pi fixtures generated with Pi's own writer: fork, clone, --fork, export and import, torn tail, v1 migration, retry with an error attempt, split-turn compaction, tool usage.
- Claude fixtures: block records disagreeing on output_tokens, fork-style subagent replaying parent uuids, non-msg_0 message ID without requestId, multi-element iterations.
- metaproc captures ported and scrubbed of timestamps, IDs and signatures; port metaproc's hygiene scanner with missing patterns; add torn-tail and rotation fixtures.
