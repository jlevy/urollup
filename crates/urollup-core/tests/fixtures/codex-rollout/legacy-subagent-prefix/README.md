# legacy-subagent-prefix

Tests Codex copied-history rules for legacy subagent rollouts without a native boundary:
exclude the inherited prefix, infer where it ends and label it inferred with a
diagnostic, and continue the child’s counters from the inherited total (design §3.4).

- **Subagent 1, parent present:** its own `session_meta`, the parent’s first six lines
  copied in a 1 ms burst (a foreign `session_meta`, turn `turn-c06-p1` and one
  `token_count`), its own `thread_settings_applied` without `thread_id` (0.150.0), and
  two own events. The boundary comes from turn IDs also in the parent.
- **Subagent 2, parent absent:** the same shape with a single copied `token_count`, a
  single-event compressed replay.
  The boundary comes from the last foreign `session_meta`, and the child is an orphan
  root.
- **Subagent 3, parent absent:** a copied prefix with no `token_count`, and own events
  400 ms apart.
- **Reconciled:** 8 requests, 2 per thread, and 43,000 tokens.
- **Naive sums:** every `last_token_usage` gives 11 requests and 55,100 tokens.
  The ccusage 20.0.20 heuristics keep subagent 2’s copied event and drop subagent 3’s
  own two events (its 1 s burst rule): 7 requests.
- **Shapes:** Codex
  [spawn copies](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/agent/control/spawn.rs#L65-L105)
  and
  [write-time timestamps](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1977-L1994);
  ccusage’s
  [replay tests](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/loader.rs#L1042-L1656)
  and
  [burst heuristic](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L84-L149);
  squares’
  [prefix cut](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/devtools/codex_log_rollup.py#L454-L495)
  and
  [synthetic builders](https://github.com/jlevy/squares/blob/f2e24e07be8c94fa3ac603c3534dce7c454da99b/packing/tests/test_codex_log_rollup.py#L15-L122).
