# progress-nested-subagent

Tests that `progress` records nesting a subagent’s assistant message are copies (design
§3.4) and that a nested copy never adds usage (§3.3).

- **Layout:** the session spawns two `Agent` calls.
  Progress line 3 nests subagent message `S1`, which also appears in
  `subagents/agent-a0000000000030001.jsonl` beside a `.meta.json` whose `toolUseId`
  names the spawning call.
  Progress line 6 nests message `S2` of agent `a0000000000030002`, whose transcript does
  not exist.
- **Reconciled:** 5 requests.
  `S1` is one request owned by the subagent thread, with the progress line as extra
  evidence; the subagent’s second request `S1b` has no progress copy.
  The session owns its own 3 requests.
- **Orphaned copy:** line 6 is not counted and yields
  `claude-nested-copy-without-original`. This follows the letter of §3.3 and is flagged
  as an open question in `expected.json`.
- **Naive sums:** top-level records alone happen to match; adding nested usage without
  deduplication (ccusage `daily`) adds 21 input, 1,400 cache-write and 73 output tokens.
- **Shapes:** the nested record follows ccusage’s
  [progress fixture](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage/src/main.rs#L425-L484);
  the `.meta.json` keys follow
  [agentfdr’s subagent tests](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/subagents.test.js).
