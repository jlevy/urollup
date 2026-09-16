# archived-rename

Tests Codex discovery of flat `archived_sessions/`, compressed archived rollouts and
source identity by thread and rollout ID rather than path (design §2.1, §3.6).

- **Archived:** parent `019f0000-0000-7000-8000-001100000001` renamed into flat
  `archived_sessions/` with its spawned child `019f0000-0000-7000-8000-001100000002` as
  `.jsonl.zst`. The child’s file name says `2026-09-09T23-30-00` local time (UTC-7), but
  its records are on 2026-09-10 UTC.
- **Leftover copy:** a byte-identical copy of the parent under `sessions/2026/09/09/`,
  as a sync tool or backup restore can leave.
- **Active:** an unrelated thread under `sessions/2026/09/10/`.
- **Reconciled:** 5 requests (3 parent, 1 child, 1 unrelated) and 18,870 tokens, the
  duplicate location reported with `codex-rollout-duplicate-location`. By UTC day: 2
  requests on 2026-09-09 and 3 on 2026-09-10.
- **Naive sum:** treating each path as a source counts the parent twice: 8 requests and
  33,510 tokens.
- **Shapes:** Codex
  [archive rename](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/archive_thread.rs#L80-L119)
  and
  [local-time names](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/recorder.rs#L1635-L1657);
  ccusage’s
  [archived discovery](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/paths.rs#L20-L116).
