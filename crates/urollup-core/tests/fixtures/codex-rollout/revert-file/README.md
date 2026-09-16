# revert-file

Tests one thread spread over two rollout files after a revert: sources are identified by
thread and rollout ID, not path (design §2.1, §3.6), and reverted history still consumed
usage (§3.3).

- **Old file:** `rollout-…-019f0000-0000-7000-8000-000900000001.jsonl`, paginated, with
  turns 1 and 2 and their records.
- **Revert file:**
  `rollout-…-019f0000-0000-7000-8000-000900000001_019f0000-0000-7000-8000-000900000002.jsonl`.
  Its `session_meta` keeps thread ID `019f0000-0000-7000-8000-000900000001`, starts at
  ordinal 8, and has a `history_base` naming the old file’s rollout ID,
  `end_ordinal_exclusive: 8` and the byte offset after ordinal 7 (2995 bytes).
  Turn 3 replaces turn 2.
- **Reconciled:** 3 requests and 15,950 tokens, all owned by one thread; the reverted
  turn 2 response still counts.
- **Naive sums:** keeping only the last sorted path per thread (squares) finds 1
  request; re-reading the referenced prefix as part of the revert file counts turn 1
  twice.
- **Shapes:** Codex
  [revert files](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/thread-store/src/local/revert_thread.rs#L15-L18),
  [file names](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/rollout_file_name.rs#L39-L76),
  [HistoryPosition](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/protocol/src/protocol.rs#L3018-L3032)
  and
  [ordinal continuation](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/rollout/src/ordinal.rs#L16-L53).
