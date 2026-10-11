# resumed-subagent-copy

Tests design §3.4’s rule that a Claude Code subagent thread is the session its lines
name plus its agent ID, not the directory that holds its file: a byte-identical copy of
a subagent transcript under a resumed session is the original subagent, so its responses
count once with one owner.

- **Layout:** session `00000000-0000-4000-8000-001800000001` spawns subagent
  `a0000000000180001`, whose transcript and `.meta.json` sidecar sit in its `subagents/`
  directory. Session `00000000-0000-4000-8000-001800000002` resumes it: its file replays
  the original’s four lines byte for byte under the original `sessionId`, then adds its
  own turn, which spawns subagent `a0000000000180002`. Its `subagents/` directory holds
  that new subagent and a byte-identical copy of the original’s subagent transcript and
  sidecar, whose lines all still name the first session.
- **Reconciled:** 7 requests and 9,994 tokens, all owned.
  The copy belongs to thread `…001800000001/a0000000000180001`, so each of its two
  responses is one request with two identical original records, owned by that subagent,
  with no diagnostic and no empty thread for the copy’s directory.
  The resumed session’s replays are resume-replay copies of the first session’s
  responses, and its own subagent is its child.
- **Before the fix:** urollup took the thread from the directory, so both copies proved
  ownership: when the copy’s `src-` ID sorted first, both responses landed on Claude
  Code’s unowned row with `conflicting-owners`, and the copy always added an empty
  subagent thread under the resumed session.
- **Original missing:** Claude Code deletes old transcripts after 30 days by default.
  If the first session’s files are gone, the copy alone still counts the subagent’s
  responses once, owned by the same subagent thread; a Rust test removes them.
- **Naive sums:** every assistant record gives 11 requests and counts each copied or
  replayed response twice.
- **Shapes:** the record and sidecar shapes of `progress-nested-subagent`; the copy
  layout follows a local real-history observation, kept qualitative, and every value
  here is synthetic.
