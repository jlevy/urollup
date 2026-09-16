---
type: is
id: is-01m2f0te8wwa4jmh6zzamt27zb
title: Freeze sanitized fixtures and representative corpus manifest
kind: task
status: closed
priority: 2
version: 17
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2f0tewzgw6zd3vzrp2c865s
  - type: blocks
    target: is-01m2khsdqz3qndz3aetey1f5c7
  - type: blocks
    target: is-01m2ks6m5jg3tqh3dv22mqmv17
  - type: blocks
    target: is-01m2ksns5eqx65np449amxw7wd
  - type: blocks
    target: is-01m2ksntwyn71sp9tdqxrvv5wn
  - type: blocks
    target: is-01m2ksp6dstb6fwkbmy587tt8a
  - type: blocks
    target: is-01m2ksp7t02wzxrgvs75js0a7d
  - type: blocks
    target: is-01m2kspssne0kaf0ykdmt0pbe3
  - type: blocks
    target: is-01m2ksptx4kzgd5wrf4vdvaaa0
  - type: blocks
    target: is-01m2kspw1xv17v916gr7bv3943
  - type: blocks
    target: is-01m2ksqceec58vjff114xqjafr
  - type: blocks
    target: is-01m2ksqdqtvwncabnag6zkehd0
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-14T03:54:21.852Z
updated_at: 2026-09-16T03:03:47.897Z
closed_at: 2026-09-16T03:03:47.894Z
close_reason: "The public fixture corpus is frozen on milestone-0.1: 28 cases (14 claude-project, 14 codex-rollout) with expected.json in urollup-fixture-expected/v1, a README per case, the transcript sanitizer and the make fixtures-check gate with two gate probes. The consented representative corpus manifest, the other half of this bead, is split out as its own bead under milestone 0.5; gaps in case coverage stay in uro-jru4 and uro-vsfb."
resolution: null
duplicate_of: null
---
Public synthetic or sanitized fixtures, including the research brief's double-counting cases, plus a consented representative corpus manifest.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- ccusage negative fixtures: nested null in tool input, sidechain replay before its parent, gateway message-ID reuse, single-event compressed Codex replay, fork events under 1 s apart, microsecond timestamps, codex-auto-review model, cache_creation disagreeing with the flat count; adapt ccusage's fs_fixture! macro and relevant tests (MIT notice).
- Codex synthetic goldens: cumulative exec usage over resume, context-full fills, compaction estimates, info:null, multi-limit_id snapshots, legacy user fork copies, legacy and paginated subagent prefixes, revert files, .zst twins, archived renames, migrated rollouts; port squares' synthetic Codex builders (record squares repo and commit).
- Pi fixtures generated with Pi's own writer: fork, clone, --fork, export and import, torn tail, v1 migration, retry with an error attempt, split-turn compaction, tool usage.
- Claude fixtures: block records disagreeing on output_tokens, fork-style subagent replaying parent uuids, non-msg_0 message ID without requestId, multi-element iterations.
- metaproc captures ported and scrubbed of timestamps, IDs and signatures; port metaproc's hygiene scanner with missing patterns; add torn-tail and rotation fixtures.
