# workflow-subagents

Tests discovery of `subagents/workflows/<workflow>/` transcripts and a spawn edge whose
`toolUseId` lies in another subagent (design §3.2), plus `self` and `descendants` totals
(§4.2).

- **Layout:** the session’s `Agent` call spawns subagent `A`
  (`subagents/agent-a0000000000110001.jsonl`). `A`’s own `Agent` call spawns workflow
  subagent `B` under `subagents/workflows/wf_synthetic_1101/`, whose `.meta.json` has
  `spawnDepth: 2` and a `toolUseId` naming `A`’s call.
- **Reconciled:** 5 requests, none duplicated.
  The session owns 2, `A` owns 2 and `B` owns 1. `B`’s parent is `A`, so `A`’s
  descendants hold 3 requests and the session’s hold all 5.
- **Naive grouping:** tokens match, but ccusage 20.0.20 derives a second session named
  after the workflow directory, so a per-session report splits the tree.
- **Shapes:**
  [agentfdr’s workflow test](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/subagents.test.js)
  and ccusage’s
  [path derivation](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/claude/src/paths.rs#L91-L153).
